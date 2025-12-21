# Development documentation

## Getting Started with building the endpoint

### Prerequisites

- Rust version 1.85 or higher
    - macOS/Linux: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain 1.85 -y`
- CMake version 3.31.6 or higher
    - macOS: `brew install cmake`
    - Linux: (Debian/Ubuntu): `apt install cmake`
- [libclang](https://rust-lang.github.io/rust-bindgen/requirements.html#installing-clang) library 9.0 or higher.
- C++ compiler
    - macOS: `xcode-select --install` but you likely already have it if you are using `brew`
    - Linux (Debian/Ubuntu): `apt install build-essential`

This project is compatible with Linux and macOS systems.

### Building

To install TrustTunnel Endpoint, follow these steps:

1. Clone the repository:

   ```shell
   git clone https://github.com/TrustTunnel/TrustTunnel.git
   cd TrustTunnel
   ```

2. Build the binaries using Cargo:

   ```shell
   cargo build --bins --release
   ```

   or to build binaries for debug:

   ```shell
   cargo build --bins
   ```

   This command will generate the executables in the `target/release` or `target/debug` directory accordingly.

## Usage

### Setup

To quickly configure and launch the VPN endpoint, run the following commands:

```shell
make ENDPOINT_HOSTNAME="example.org" endpoint/setup  # You can skip it if you have already configured the endpoint earlier
make endpoint/run
```

Check `Makefile` for available configuration variables.

These commands perform the following actions:

1. Build the wizard and endpoint binaries.

2. Configure the endpoint to listen to all network interfaces for TCP/UDP packets on
   port number 443.

3. Generate self-signed certificate/private key pair in the current directory under `certs/`.

4. Store all the required settings in `vpn.toml` and `hosts.toml` files.

5. Start the endpoint.

Alternatively, you can run the endpoint in a docker container:

```shell
docker build -t trusttunnel-endpoint:latest . # build an image

docker run -it trusttunnel-endpoint:latest --name trusttunnel-endpoint # create docker container and start it in an interactive mode

docker start -i trusttunnel-endpoint # if you need to start your vpn endpoint again
```

### Customized Configuration

For a more customized configuration experience, run the following commands:

```shell
make endpoint/build-wizard  # If you skipped the previous chapter
cargo run --bin setup_wizard  # Launches a dialogue session allowing you to tweak the settings
cargo run --bin trusttunnel_endpoint -- <lib-settings> <hosts-settings>  # File names depend on the previous step
```

For additional details about the binary, refer to the [endpoint/README.md](./endpoint/README.md)
file.
