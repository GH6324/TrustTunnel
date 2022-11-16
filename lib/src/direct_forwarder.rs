use std::io;
use std::net::IpAddr;
use std::sync::Arc;
use async_trait::async_trait;
use crate::{authentication, core, datagram_pipe, downstream, forwarder, log_utils, tunnel, udp_forwarder};
use crate::forwarder::Forwarder;
use crate::tcp_forwarder::TcpForwarder;


pub(crate) struct DirectForwarder {
    context: Arc<core::Context>,
}

impl DirectForwarder {
    pub fn new(
        context: Arc<core::Context>,
    ) -> Self {
        Self {
            context,
        }
    }
}

impl Forwarder for DirectForwarder {
    fn tcp_connector(&self) -> Box<dyn forwarder::TcpConnector> {
        Box::new(TcpForwarder::new(self.context.settings.clone()))
    }

    fn datagram_mux_authenticator(&self) -> Box<dyn forwarder::DatagramMultiplexerAuthenticator> {
        struct Dummy;

        #[async_trait]
        impl forwarder::DatagramMultiplexerAuthenticator for Dummy {
            async fn check_auth(
                self: Box<Self>,
                _: IpAddr,
                _: &'_ str,
                _: authentication::Source<'_>,
                _: Option<&'_ str>,
            ) -> Result<(), tunnel::ConnectionError> {
                Ok(())
            }
        }

        Box::new(Dummy)
    }

    fn make_udp_datagram_multiplexer(
        &self,
        id: log_utils::IdChain<u64>,
        meta: forwarder::UdpMultiplexerMeta,
    ) -> io::Result<(
        Arc<dyn forwarder::UdpDatagramPipeShared>,
        Box<dyn datagram_pipe::Source<Output = forwarder::UdpDatagramReadStatus>>,
        Box<dyn datagram_pipe::Sink<Input = downstream::UdpDatagram>>,
    )> {
        assert!(meta.auth.is_none());
        udp_forwarder::make_multiplexer(id)
    }

    fn make_icmp_datagram_multiplexer(
        &self, id: log_utils::IdChain<u64>
    ) -> io::Result<Option<(
        Box<dyn datagram_pipe::Source<Output = forwarder::IcmpDatagram>>,
        Box<dyn datagram_pipe::Sink<Input = downstream::IcmpDatagram>>,
    )>> {
        self.context.icmp_forwarder.as_ref()
            .map(|x| x.make_multiplexer(id))
            .transpose()
    }
}
