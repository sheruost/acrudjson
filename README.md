# ACRUDJSON

[![Cargo](https://img.shields.io/crates/v/acrudjson.svg)](
https://crates.io/crates/acrudjson)
![Minimum rustc version](https://img.shields.io/badge/rustc-1.62.1+-blue.svg)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/sheruost/acrudjson)

Atomic CRUD API for arithmetic operations with astronomically large floating point numbers based on JSON-RPC [specification](https://www.jsonrpc.org).

## Features

- **Parallelism**: TODO
- **Asynchronous**: `async` support provided by `tokio-rs`
- **Simple**: Mimimum dependencies even with `std` feature.

## TODO

- [ ] build an example of cli-tool under `examples/client.rs`
- [ ] support JSON-RPC 2.0 (https://www.jsonrpc.org/specification)
- [ ] support `no_std` feature
- [ ] provide more arithmetic methods

## License

**AcrudJSON** is licensed under either of:
- Apache License 2.0 (http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (http://opensource.org/licenses/MIT)
