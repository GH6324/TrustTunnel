use std::io;
use std::io::ErrorKind;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::net::{TcpListener, UdpSocket};
use crate::direct_forwarder::DirectForwarder;
use crate::{authentication, http_ping_handler, log_id, log_utils, metrics, protocol_selector, utils};
use crate::protocol_selector::{Protocol, PingProtocol, ServiceMessengerProtocol, TunnelProtocol};
use crate::forwarder::Forwarder;
use crate::http1_codec::Http1Codec;
use crate::http2_codec::Http2Codec;
use crate::http3_codec::Http3Codec;
use crate::http_codec::HttpCodec;
use crate::http_downstream::HttpDownstream;
use crate::icmp_forwarder::IcmpForwarder;
use crate::metrics::Metrics;
use crate::quic_multiplexer::{QuicMultiplexer, QuicSocket};
use crate::settings::{ForwardProtocolSettings, ListenProtocolSettings, Settings};
use crate::shutdown::Shutdown;
use crate::socks5_forwarder::Socks5Forwarder;
use crate::tls_listener::{TlsAcceptor, TlsListener};
use crate::tunnel::Tunnel;


pub struct Core {
    context: Arc<Context>,
}


pub(crate) struct Context {
    pub settings: Arc<Settings>,
    pub icmp_forwarder: Option<Arc<IcmpForwarder>>,
    pub shutdown: Arc<Mutex<Shutdown>>,
    pub metrics: Arc<Metrics>,
    next_client_id: Arc<AtomicU64>,
    next_tunnel_id: Arc<AtomicU64>,
}

impl Core {
    pub fn new(
        settings: Settings,
        shutdown: Arc<Mutex<Shutdown>>,
    ) -> Self {
        let settings = Arc::new(settings);

        Self {
            context: Arc::new(Context {
                settings: settings.clone(),
                icmp_forwarder: if settings.icmp.is_none() {
                    None
                } else {
                    Some(Arc::new(IcmpForwarder::new(settings)))
                },
                shutdown,
                metrics: Metrics::new().unwrap(),
                next_client_id: Default::default(),
                next_tunnel_id: Default::default(),
            }),
        }
    }

    /// Run an endpoint instance inside the caller provided asynchronous runtime.
    /// In this case some of the endpoint settings are ignored as they do not have any sense,
    /// like [`Settings::threads_number`].
    pub async fn listen_async(&mut self) -> io::Result<()> {
        let listen_tcp = async {
            self.listen_tcp().await
                .map_err(|e| io::Error::new(e.kind(), format!("TCP listener failure: {}", e)))
        };

        let listen_udp = async {
            self.listen_udp().await
                .map_err(|e| io::Error::new(e.kind(), format!("UDP listener failure: {}", e)))
        };

        let listen_icmp = async {
            self.listen_icmp().await
                .map_err(|e| io::Error::new(e.kind(), format!("ICMP listener failure: {}", e)))
        };

        let listen_metrics = async {
            metrics::listen(self.context.clone(), log_utils::IdChain::empty()).await
                .map_err(|e| io::Error::new(e.kind(), format!("Metrics listener failure: {}", e)))
        };

        let (mut shutdown_notification, _shutdown_completion) = {
            let shutdown = self.context.shutdown.lock().unwrap();
            (
                shutdown.notification_handler(),
                shutdown.completion_guard()
                    .ok_or_else(|| io::Error::new(ErrorKind::Other, "Shutdown is already submitted"))?
            )
        };

        tokio::select! {
            x = shutdown_notification.wait() => {
                x.map_err(|e| io::Error::new(ErrorKind::Other, format!("{}", e)))
            },
            x = futures::future::try_join4(
                listen_tcp,
                listen_udp,
                listen_icmp,
                listen_metrics,
            ) => x.map(|_| ()),
        }
    }

    /// Run an endpoint instance in a blocking way.
    /// This one will set up its own asynchronous runtime.
    pub fn listen(&mut self) -> io::Result<()> {
        let runtime = {
            let context = self.context.clone();
            let threads_num = context.settings.threads_number;
            tokio::runtime::Builder::new_multi_thread()
                .enable_io()
                .enable_time()
                .worker_threads(threads_num)
                .build()?
        };

        let _guard = runtime.enter();

        runtime.block_on(async {
            self.listen_async().await
        })
    }

    async fn listen_tcp(&self) -> io::Result<()> {
        let settings = self.context.settings.clone();
        let has_tcp_based_codec = settings.listen_protocols.iter()
            .any(|x| match x {
                ListenProtocolSettings::Http1(_) | ListenProtocolSettings::Http2(_) => true,
                ListenProtocolSettings::Quic(_) => false,
            });

        let tcp_listener = TcpListener::bind(settings.listen_address).await?;
        info!("Listening to TCP {}", settings.listen_address);

        let tls_listener = Arc::new(TlsListener::new(self.context.settings.clone()));
        loop {
            let client_id = log_utils::IdChain::from(log_utils::IdItem::new(
                log_utils::CLIENT_ID_FMT, self.context.next_client_id.fetch_add(1, Ordering::Relaxed)
            ));
            let stream = match tcp_listener.accept().await
                .and_then(|(s, a)| { s.set_nodelay(true)?; Ok((s, a)) })
            {
                Ok((stream, addr)) => if has_tcp_based_codec {
                    log_id!(debug, client_id, "New TCP client: {}", addr);
                    stream
                } else {
                    continue; // accept just for pings
                }
                Err(e) => {
                    log_id!(debug, client_id, "TCP connection failed: {}", e);
                    continue;
                }
            };

            tokio::spawn({
                let context = self.context.clone();
                let tls_listener = tls_listener.clone();
                async move {
                    let handshake_timeout = context.settings.tls_handshake_timeout;
                    match tokio::time::timeout(handshake_timeout, tls_listener.listen(stream))
                        .await
                        .unwrap_or_else(|_| Err(io::Error::from(ErrorKind::TimedOut)))
                    {
                        Ok(stream) => Core::on_new_tls_connection(context.clone(), stream, client_id).await,
                        Err(e) => log_id!(debug, client_id, "TLS handshake failed: {}", e),
                    }
                }
            });
        }
    }

    async fn listen_udp(&self) -> io::Result<()> {
        let settings = self.context.settings.clone();
        if !settings.listen_protocols.iter()
            .any(|x| match x {
                ListenProtocolSettings::Http1(_) | ListenProtocolSettings::Http2(_) => false,
                ListenProtocolSettings::Quic(_) => true,
            })
        {
            return Ok(());
        }

        let socket = UdpSocket::bind(settings.listen_address).await?;
        info!("Listening to UDP {}", settings.listen_address);

        let mut quic_listener = QuicMultiplexer::new(
            settings,
            socket,
            self.context.next_client_id.clone(),
        );

        loop {
            let socket = quic_listener.listen().await?;

            tokio::spawn({
                let context = self.context.clone();
                let socket_id = socket.id();
                async move {
                    log_id!(debug, socket_id, "New QUIC connection");
                    Self::on_new_quic_connection(context, socket, socket_id).await;
                }
            });
        }
    }

    async fn listen_icmp(&self) -> io::Result<()> {
        let forwarder = match &self.context.icmp_forwarder {
            None => return Ok(()),
            Some(x) => x.clone(),
        };

        forwarder.listen().await
    }

    async fn on_new_tls_connection(context: Arc<Context>, acceptor: TlsAcceptor, client_id: log_utils::IdChain<u64>) {
        let sni = match acceptor.sni() {
            Some(s) => s,
            None => {
                log_id!(debug, client_id, "Drop TLS connection due to absence of SNI");
                return;
            }
        };

        let alpn = match acceptor.alpn().map(String::from_utf8) {
            Some(Ok(p)) => Some(p),
            Some(Err(e)) => {
                log_id!(debug, client_id, "Drop TLS connection due to malformed ALPN: {:?} (error: {})",
                    acceptor.alpn().unwrap(), e);
                return;
            }
            None => None,
        };

        let core_settings = context.settings.clone();
        let proto =
            match protocol_selector::select(&core_settings, alpn.as_deref(), &sni) {
                Ok(Protocol::Tunnel(TunnelProtocol::Http3))
                | Ok(Protocol::Ping(PingProtocol::Http3))
                | Ok(Protocol::ServiceMessenger(ServiceMessengerProtocol::Http3))
                => {
                    log_id!(debug, client_id, "Unexpected connection protocol - dropping tunnel");
                    return;
                }
                Ok(x) => x,
                Err(e) => {
                    log_id!(debug, client_id, "Dropping tunnel due to error: {}", e);
                    return;
                }
            };
        log_id!(trace, client_id, "Selected protocol: {:?}", proto);

        let stream = match acceptor.accept(proto, &client_id).await {
            Ok(s) => {
                log_id!(debug, client_id, "New TLS client: {:?}", s);
                s
            }
            Err(e) => {
                log_id!(debug, client_id, "TLS connection failed: {}", e);
                return;
            }
        };

        match proto {
            Protocol::Tunnel(protocol) => {
                let tunnel_id = client_id.extended(log_utils::IdItem::new(
                    log_utils::TUNNEL_ID_FMT, context.next_tunnel_id.fetch_add(1, Ordering::Relaxed)
                ));

                Self::on_tunnel_request(
                    context,
                    protocol,
                    match protocol {
                        TunnelProtocol::Http1 => Box::new(Http1Codec::new(
                            core_settings, stream, tunnel_id.clone(),
                        )),
                        TunnelProtocol::Http2 => Box::new(Http2Codec::new(
                            core_settings, stream, tunnel_id.clone(),
                        )),
                        TunnelProtocol::Http3 => unreachable!(),
                    },
                    &sni,
                    tunnel_id,
                ).await
            }
            Protocol::Ping(protocol) => http_ping_handler::listen(
                match protocol {
                    PingProtocol::Http1 => Box::new(Http1Codec::new(
                        core_settings, stream, client_id.clone(),
                    )),
                    PingProtocol::Http2 => Box::new(Http2Codec::new(
                        core_settings, stream, client_id.clone(),
                    )),
                    PingProtocol::Http3 => unreachable!(),
                },
                client_id,
            ).await,
            Protocol::ServiceMessenger(_) => { todo!() }
        }
    }

    async fn on_new_quic_connection(
        context: Arc<Context>,
        socket: QuicSocket,
        client_id: log_utils::IdChain<u64>,
    ) {
        let core_settings = context.settings.clone();

        let alpn = match String::from_utf8(socket.alpn()) {
            Ok(x) => x,
            Err(e) => {
                log_id!(debug, client_id, "Drop QUIC connection due to malformed ALPN: {} (error: {})",
                    utils::hex_dump(&socket.alpn()), e);
                return;
            }
        };

        let sni = socket.server_name().unwrap_or_default();
        let proto =
            match protocol_selector::select(&core_settings, Some(&alpn), &sni) {
                Ok(x) if x == Protocol::Tunnel(TunnelProtocol::Http3)
                    || x == Protocol::Ping(PingProtocol::Http3)
                    || x == Protocol::ServiceMessenger(ServiceMessengerProtocol::Http3)
                => x,
                Ok(x) => {
                    log_id!(debug, client_id, "Unexpected connection protocol ({:?}) - dropping tunnel", x);
                    return;
                }
                Err(e) => {
                    log_id!(debug, client_id, "Dropping tunnel due to error: {}", e);
                    return;
                }
            };
        log_id!(trace, client_id, "Selected protocol: {:?}", proto);

        match proto {
            Protocol::Tunnel(protocol) => {
                let tunnel_id = client_id.extended(log_utils::IdItem::new(
                    log_utils::TUNNEL_ID_FMT, context.next_tunnel_id.fetch_add(1, Ordering::Relaxed)
                ));

                Self::on_tunnel_request(
                    context,
                    protocol,
                    Box::new(Http3Codec::new(socket, tunnel_id.clone())),
                    &sni,
                    tunnel_id,
                ).await
            }
            Protocol::Ping(_) => http_ping_handler::listen(
                Box::new(Http3Codec::new(socket, client_id.clone())),
                client_id,
            ).await,
            Protocol::ServiceMessenger(_) => { todo!() }
        }
    }

    async fn on_tunnel_request(
        context: Arc<Context>,
        protocol: TunnelProtocol,
        codec: Box<dyn HttpCodec>,
        server_name: &str,
        tunnel_id: log_utils::IdChain<u64>,
    ) {
        let _metrics_guard = Metrics::client_sessions_counter(context.metrics.clone(), protocol);

        let authenticated = server_name != context.settings.tunnel_tls_host_info.hostname
            && match context.settings.authenticator.authenticate(
                utils::scan_sni_authentication(server_name, &context.settings.tunnel_tls_host_info.hostname).unwrap(),
                &tunnel_id
            ).await {
                authentication::Status::Pass => true,
                authentication::Status::Reject => {
                    log_id!(debug, tunnel_id, "SNI authentication failed");
                    return;
                }
            };

        log_id!(debug, tunnel_id, "New tunnel for client");
        let mut tunnel = Tunnel::new(
            context.clone(),
            Box::new(HttpDownstream::new(context.settings.clone(), codec)),
            Self::make_forwarder(context),
            authenticated,
            tunnel_id.clone(),
        );

        log_id!(trace, tunnel_id, "Listening for client tunnel");
        match tunnel.listen().await {
            Ok(_) => log_id!(debug, tunnel_id, "Tunnel stopped gracefully"),
            Err(e) => log_id!(debug, tunnel_id, "Tunnel stopped with error: {}", e),
        }
    }

    fn make_forwarder(context: Arc<Context>) -> Box<dyn Forwarder> {
        match &context.settings.forward_protocol {
            ForwardProtocolSettings::Direct(_) => Box::new(DirectForwarder::new(context)),
            ForwardProtocolSettings::Socks5(_) => Box::new(Socks5Forwarder::new(context)),
        }
    }
}

#[cfg(test)]
impl Default for Context {
    fn default() -> Self {
        Self {
            settings: Arc::new(Settings::default()),
            icmp_forwarder: None,
            shutdown: Shutdown::new(),
            metrics: Metrics::new().unwrap(),
            next_client_id: Default::default(),
            next_tunnel_id: Default::default(),
        }
    }
}
