# Changelog

## [0.2.1] - 2026-10-11

### Fixed

- A batch element written as a JSON array was run as a request. It now gets
  an Invalid Request reply, like any other element that isn't an object.
- `Op::eval` could be called from other crates, skipping the `MAX_DIGITS` and
  `MAX_SCALE` checks. Operations now run only through `Op::apply`.
- `Error::Storage` printed its cause in `Display` and also returned it from
  `source()`, so error chains showed it twice. `Display` now says only
  "storage error".
- `cargo test --no-default-features` failed on a doc example that needs `std`.

## [0.2.0] - 2026-10-10

A rewrite. The 0.1 API is gone, and databases written by 0.1 can't be read.

### Added

- **JSON-RPC 2.0**, alongside 1.0, chosen per request: batches,
  notifications, and named or positional params.
- **`no_std` support** for the protocol side (`alloc` only). Bring your own
  storage by implementing `Backend`.
- **`client` module**: one type per method (`Create`, `Read`, `Update`,
  `Delete`, `Compute`, `ComputeInto`), `Batch`, and `Response`.
- **UDP and TCP servers** behind the `tokio` feature, generic over any `Backend`.
- `Backend` for `&B`, `Box<B>` and `Arc<B>`, so a shared `Arc<Store>` works anywhere.
- `String` keys and integer literals as operands: `store.compute(ns, Multiply, &key, &2)`.
- `RpcError::into_owned`, to `?` a server error into `Box<dyn Error>`.
- `From<Error>` for `std::io::Error`.
- **`compute_into`**: saves the result of an operation in the same
  transaction that read its operands.
- **Number limits** (`MAX_DIGITS`, `MAX_SCALE`), checked before allocating.
- **Error codes** `-32001` to `-32004`, on top of the standard JSON-RPC ones.

### Changed

- **Edition 2024, nightly only.** The TCP line reader needs Polonius, and the
  server reads param names from the method types by reflection (`type_info`).
- **Storage**: sled is replaced by redb (`Store`). Numbers are stored as
  binary digits plus a scale instead of decimal text.
- **Design**: operations (`op::Add`, …) and protocol versions (`V1`, `V2`)
  are types behind traits (`Backend`, `Op`, `Operand`, `Version`, `Call`)
  instead of enums.
- **Parsing**: params are borrowed from the input instead of cloned into
  `Vec<String>`, and numbers are parsed from their exact JSON text.
- **Replies** are appended to a buffer the caller owns and reuses.
- **Wire format**: messages no longer carry a CRC32 trailer. Over TCP, each
  message is one line.
- A `"jsonrpc": "1.0"` member is now rejected; JSON-RPC 1.0 never defined it.
- **Dependencies**: lower bounds instead of exact patch versions.
- `license` uses SPDX syntax: `MIT OR Apache-2.0`.

### Removed

- `prelude::v1`, `RequestBuilder`, `ResponseBuilder`, `ReqBody`, `RespBody`.
- `ConnectionPool`, `UserDatabase`.
- `Method`, `BinaryOps`, `Param`, `JsonInternal`.
- `ClientError`, `ServerError`, `ErrorMsg`; replaced by `Error`.
- Dependencies on `sled`, `thiserror`, `zerocopy`, `crc32fast` and `log`.

### Fixed

- The server no longer panics on an unknown method, a division by zero, or
  malformed input. Each is answered with a JSON-RPC error.

[0.2.1]: https://github.com/sheruost/acrudjson/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/sheruost/acrudjson/compare/v0.1.2...v0.2.0
