<p align="center">
<picture>
<source media="(prefers-color-scheme: dark)" srcset="https://cdn.adguardcdn.com/website/github.com/TrustTunnel/logo_dark.svg" width="300px" alt="TrustTunnel" />
<img src="https://cdn.adguardcdn.com/website/github.com/TrustTunnel/logo_light.svg" width="300px" alt="TrustTunnel" />
</picture>
</p>

<p align="center"><a href="https://github.com/TrustTunnel/TrustTunnelClient">Console client</a>
  · <a href="https://github.com/TrustTunnel/TrustTunnelFlutterClient">Flutter-based app</a>
  · <a href="https://agrd.io/ios_trusttunnel">App store</a>
  · <a href="https://agrd.io/android_trusttunnel">Play store</a>
</p>

---

## Table of Contents

- [Introduction](#introduction)
- [Server Features](#server-features)
- [Client Features](#client-features)
- [Quick start](#quick-start)
    - [Endpoint setup](#endpoint-setup)
        - [Install the endpoint](#install-the-endpoint)
        - [TrustTunnel Flutter Client 1.0 warning](#trusttunnel-flutter-client-1.0-warning)
        - [Endpoint configuration](#endpoint-configuration)
        - [Running endpoint](#running-endpoint)
        - [Export client configuration](#export-client-configuration)
    - [Client setup](#client-setup)
        - [Install the client](#install-the-client)
        - [Client configuration](#client-configuration)
        - [Running client](#running-client)
- [Additional documentation](#additional-documentation)
- [Roadmap](#roadmap)
- [License](#license)

---

## Introduction

Welcome to the TrustTunnel repository!

TrustTunnel is a free, fast, secure, and fully self-hosted VPN solution powered by its own unique VPN protocol.

The TrustTunnel project includes the VPN endpoint (this repository), the [library and CLI for the client](https://github.com/TrustTunnel/TrustTunnelClient), and the [GUI application](https://github.com/TrustTunnel/TrustTunnelFlutterClient).

## Server Features

- **VPN Protocol**: The library implements the VPN protocol compatible
  with HTTP/1.1, HTTP/2, and QUIC.
  By mimicking regular network traffic, it becomes more difficult for government regulators to
  detect and block.

- **Flexible Traffic Tunneling**: TrustTunnel can tunnel TCP, UDP, and ICMP traffic to and
  from the client.

- **Platform Compatibility**: The server is compatible with Linux and macOS. The client is available for Android, Apple, Windows, and Linux.

---

## Client Features

- **Traffic Tunneling**: The library is capable of tunneling TCP, UDP, and ICMP traffic from the
  client to the endpoint and back.

- **Cross-Platform Support**: It supports Linux, macOS, and Windows platforms, providing a
  consistent experience across different operating systems.

- **System-Wide Tunnel and SOCKS5 Proxy**: It can be set up as a system-wide tunnel, utilizing a
  virtual network interface, as well as a SOCKS5 proxy.

- **Split Tunneling**: The library supports split tunneling, allowing users to exclude connections
  to certain domains or hosts from routing through the VPN endpoint, or vice versa, only routing
  connections to specific domains or hosts through the endpoint based on an exclusion list.

- **Custom DNS Upstream**: Users can specify a custom DNS upstream, which is used for DNS queries
  routed through the VPN endpoint.

---

## Quick start

### Endpoint setup

#### Install the endpoint

An installation script is available that can be run with the following command:

```bash
curl -fsSL https://raw.githubusercontent.com/TrustTunnel/TrustTunnel/refs/heads/master/scripts/install.sh | sh -s -
```

The installation script will download the prebuilt package from the latest GitHub release for the appropriate system architecture and unpack it to `/opt/trusttunnel`. The output directory could be overridden by specifying `-o DIR` flag at the end of the command above.

> Currently only `linux-x86_64` and `linux-aarch64` architectures are provided for the prebuilt packages.

#### TrustTunnel Flutter Client 1.0 warning

> TrustTunnel Flutter Client **doesn't support** self-signed certificates yet. If you want to use the TrustTunnel Flutter Client, you should have a valid certificate issued by a publicly trusted Certificate Authority (CA) associated with a registered domain for the IP address of the endpoint. Otherwise, the TrustTunnel Flutter Client will be unable to connect to the endpoint.

#### Endpoint configuration

The installation directory contains `setup_wizard` binary that helps generate the config files required for the endpoint to run:

```bash
cd /opt/trusttunnel/
./setup_wizard -h
```

The setup wizard supports interactive mode, so you could run it and it will ask for data required for endpoint configuration.

```bash
cd /opt/trusttunnel/
./setup_wizard
```

The wizard will ask for the following fields, some of them have the default values you could safely use:

- **The address to listen on** - specify the address for the endpoint to listen on. Use the default `0.0.0.0:443` if you want the endpoint to listen on port 443 (HTTPS) on all interfaces.
- **Path to credentials file** - path where the user credentials for authorization will be stored.
- **Username** - the username the user will use for authorization.
- **Password** - the user's password.
- **Add one more user?** - select `yes` if you want to add more users, or `no` to continue the configuration process.
- **Path to the rules file** - path to store the filtering rules.
- **Connection filtering rules** - you can add rules that the endpoint will use to allow or disallow user's connections based on:
    - Client IP address
    - TLS random prefix
    - TLS random with mask

  Press `n` to allow all connections.
- **Path to a file to store the library settings** - path to store the main endpoint configuration file.
- **Generate a self-signed certificate?** - the endpoint could generate the self-signed certificate to use for the HTTPS connection.

  Press `n` if you are going to use the TrustTunnel Flutter Client and specify the key/cert pair of the certificate issued by a CA.
  > If you have a registered domain, the recommended setup is to generate the publicly trusted key/certificate pair using ACME client, e.g. `certbot`, and specify it during this step. Please, refer to `certbot` [documentation](https://eff-certbot.readthedocs.io/en/stable/using.html#getting-certificates-and-choosing-plugins)
- **Path to a file to store the TLS hosts settings** - path to store the TLS host settings file.

At this point all required configuration files are created and saved on disk.

> The settings files created by the Setup Wizard contain almost all available settings,
> including descriptions.
> You can freely customize them if you are confident in your understanding of the configuration.

#### Running endpoint

The installed package contains the systemd service template, called `trusttunnel.service.template`.

This template can be used to set up the endpoint as a systemd service:

> NOTE: the template file assumes that the TrustTunnel Endpoint binary and all its configuration files are located in `/opt/trusttunnel` and have the default file names. Modify the template if you have used the different paths.

```bash
cd /opt/trusttunnel/
mv trusttunnel.service.template trusttunnel.service
sudo ln -s trusttunnel.service /etc/systemd/system/trusttunnel.service
sudo systemctl daemon-reload
sudo systemctl enable --now trusttunnel
```

#### Export client configuration

The endpoint binary is capable of generating the client configuration for a particular user.

This configuration contains all necessary information that is required to connect to the endpoint.

To generate the configuration, run the following command:

```shell
# <client_name> - name of the client those credentials will be included in the configuration
# <public_ip_and_port> - `ip:port` that the user will use to connect to the endpoint
cd /opt/trusttunnel/
./trusttunnel_endpoint vpn.toml hosts.toml -c <client_name> -a <public_ip_and_port>
```

This will print the configuration with the credentials for the client named `<client_name>`.

The generated client configuration could be used to set up the TrustTunnel Flutter Client, refer to the documentation in the [appropriate repository](https://github.com/TrustTunnel/TrustTunnelFlutterClient/blob/master/README.md#server-configuration).

Congratulations! You've done setting up the endpoint!

### Client setup

#### Install the client

You have a choice to use a [CLI client](https://github.com/TrustTunnel/TrustTunnelClient) or a [GUI client](https://github.com/TrustTunnel/TrustTunnelFlutterClient)

To install the CLI client, run the following command:

```bash
curl -fsSL https://raw.githubusercontent.com/TrustTunnel/TrustTunnelClient/refs/heads/master/scripts/install.sh | sh -s -
```

The installation script will download the prebuilt package from the latest GitHub release for the appropriate system architecture and unpack it to `/opt/trusttunnel_client`. The output directory could be overridden by specifying `-o DIR` flag at the end of the command above.

> Install script supports x86_64, aarch64, armv7, mips and mipsel architectures for linux and arm64 and x86_64 for macos.

#### Client configuration

The installation directory contains `setup_wizard` binary that helps generate the config files required for the client to run:

```bash
cd /opt/trusttunnel_client/
./setup_wizard -h
```

To configure the client to use the config that was generated by endpoint, run the following command:

```bash
./setup_wizard --mode non-interactive \
     --endpoint_config <endpoint_config>
     --settings trusttunnel_client.toml
```

where `<endpoint_config>` is path to a config generated by the endpoint.

`trusttunnel_client.toml` will contain all required configuration for the client.

#### Running client

To run the client execute the following command:

```bash
cd /opt/trusttunnel_client/
sudo ./trusttunnel_client -c trusttunnel_client.toml
```

`sudo` is required to set up the routes and tun interface.

## Additional documentation

Refer to the [DEVELOPMENT.md](DEVELOPMENT.md) for the more detailed documentation, including instructions to build the project from source.

## Roadmap

While our VPN currently supports tunneling TCP/UDP/ICMP traffic, we plan to add support for
peer-to-peer communication between clients.
Stay tuned for this feature in upcoming releases.

## License

This project is licensed under the Apache 2.0 License. See [LICENSE.md](LICENSE.md) for details.
