# ACRUDJSON

[![Cargo](https://img.shields.io/crates/v/acrudjson.svg)](
https://crates.io/crates/acrudjson)
[![Documentation](https://docs.rs/acrudjson/badge.svg)](https://docs.rs/acrudjson/)
![Minimum rustc version](https://img.shields.io/badge/rustc-1.61.0+-blue.svg)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/sheruost/acrudjson)

Atomic CRUD API for arithmetic operations with astronomically large floating point numbers based on JSON-RPC [specification](https://www.jsonrpc.org).

## Features

- **Parallelism**: TODO
- **Asynchronous**: `async` support provided by `tokio-rs`
- **Simple**: Mimimum dependencies even with `std` feature.

## TODO

- [ ] support JSON-RPC 2.0 (https://www.jsonrpc.org/specification)
- [ ] support `no_std` feature once ([`sled-rs`](https://sled.rs)) reaches `v1.0.0`
- [ ] provide more arithmetic methods

## License

**AcrudJSON** is licensed under either of:
- Apache License 2.0 (http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (http://opensource.org/licenses/MIT)
