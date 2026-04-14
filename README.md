# Project Introduction
A Rust program that forwards data from one serial port to multiple Unix sockets based on a custom protocol (header + data + checksum).

## Forwarding Rules
- Each header corresponds to a Unix socket. Users edit `config.toml` to configure which header maps to which socket.
- Packets failing CRC validation are discarded (optional).
- If all conditions are satisfied, the header and the data segment are written to the corresponding Unix socket.

## Technology Stack
- Async runtime: Tokio
- Data parsing: Codec from tokio-util + Bytes
- Logging: log + env_logger
- Configuration: TOML
- Serial port: tokio-serial
- Error handling: thiserror (structured, recoverable errors) + anyhow (errors that only need to be printed)

## Quick Start
```sh
Usage: serial_mux [OPTIONS]

Options:
  -c, --config <CONFIG>  Config file to use, defaults to ./config.toml
  -h, --help             Print help
```

```sh
# Release build: slower compilation, faster execution
cargo run -r

# Debug build: faster compilation, slower execution
cargo run
```

## Configuration
See `config.toml` for details.