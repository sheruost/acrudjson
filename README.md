# acrudjson

[![crates.io](https://img.shields.io/crates/v/acrudjson.svg)](https://crates.io/crates/acrudjson)
[![docs.rs](https://docs.rs/acrudjson/badge.svg)](https://docs.rs/acrudjson)
![rustc](https://img.shields.io/badge/rustc-nightly--2026--10--09-orange.svg)
![borrowck](https://img.shields.io/badge/borrowck-Polonius-blueviolet.svg)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> I first published acrudjson in September 2023. Its GitHub repository was later
> deleted, but crates.io kept the source. In October 2026 I recovered 0.1.2
> from the published crate (tagged [`v0.1.2`](https://github.com/sheruost/acrudjson/tree/v0.1.2)) and rebuilt it
> on today's nightly. Same idea, Rust then and now.

acrudjson stores very large (or very small) decimal numbers in [redb] and lets
you create, read, update, delete and do arithmetic on them over **JSON-RPC 1.0
or 2.0**. No digit is ever lost to `f64` along the way.

**Contents:** [Nightly features](#nightly-features) · [no_std](#no_std) ·
[Changelog](CHANGELOG.md) · [Docs](https://docs.rs/acrudjson)

```rust
use acrudjson::op::Multiply;
use acrudjson::{Backend, BigDecimal, Store};

let store = Store::open("numbers.redb")?;
store.create("lab", "G", &"6.6743e-11".parse()?)?;

// Call it from Rust. A `str` is a key, a `BigDecimal` is a literal.
let r = store.compute("lab", Multiply, "G", &BigDecimal::from(2))?;

// Or hand it JSON-RPC bytes. The reply is appended to `out`.
let mut out = Vec::new();
store.handle("lab", br#"{"jsonrpc":"2.0","method":"read","params":["G"],"id":1}"#, &mut out);
// {"jsonrpc":"2.0","result":6.6743E-11,"id":1}
```

> [!TIP]
> There's a small server and client in `examples/`:
>
> ```sh
> cargo run --example server --features tokio   # UDP and TCP on 127.0.0.1:9999
> cargo run --example client --features tokio
> ```

## Nightly features

acrudjson started on nightly in 2023 and it still lives there. Some things
that were experiments back then are much closer now.

> [!WARNING]
> **Polonius and reflection make the whole crate nightly-only.** Build with any
> recent nightly:
>
> ```sh
> cargo +nightly test --all-features
> ```

### Polonius

The TCP server reads lines into one buffer and hands out each line as a slice
of it, so nothing is copied:

```rust
pub async fn next<R: AsyncRead + Unpin>(&mut self, reader: &mut R) -> io::Result<Option<&[u8]>> {
    loop {
        if let Some(line) = self.buffered_line() {
            return Ok(Some(line));
        }
        self.compact();
        // grow the buffer if it's full, then read more
    }
}
```

**Stable Rust rejects this, even today (1.98).** Because `line` *might* be
returned, the current borrow checker keeps `self` borrowed for the rest of the
function, even on the path where nothing was returned. Polonius looks at each
path on its own and sees that the borrow is gone by the time we reach
`compact()`.

<details>
<summary>The error on stable</summary>

```text
error[E0499]: cannot borrow `*self` as mutable more than once at a time
149 |             if let Some(line) = self.buffered_line() {
    |                                 ---- first mutable borrow occurs here
150 |                 return Ok(Some(line));
    |                        -------------- returning this value requires that `*self` is borrowed for `'1`
...
153 |             self.compact();
    |             ^^^^ second mutable borrow occurs here
```

</details>

The same loop on two nightlies:

| | without flag | with `-Zpolonius` |
|---|:---:|:---:|
| **nightly-2023-09-29** | ❌ | ✅ |
| **nightly-2026-10-09** | ✅ | ✅ |

So in 2023 it was an experiment you had to opt into. Now the nightly compiler
accepts it as is. The repo still passes `-Zpolonius=next` (in
`.cargo/config.toml`) to make that clear.

> [!NOTE]
> Nightly moves, so the flag may be renamed or dropped one day, most likely
> once Polonius becomes the default on stable. If a future nightly rejects it,
> delete the line from `.cargo/config.toml`.

### Reflection

Every method's params are written **once**, as the fields of a type in
[`src/call.rs`](src/call.rs):

```rust
pub struct Create<'a> {
    pub key: &'a str,
    pub value: &'a BigDecimal,
}
```

The server reads those names at compile time with the nightly `type_info`
feature, the closest thing Rust has to C++26's `^^T`:

```rust
const CREATE: [&str; 2] = field_names::<Create<'static>, 2>(); // ["key", "value"]
```

<details>
<summary><code>field_names</code></summary>

```rust
const fn field_names<T: ?Sized + 'static, const N: usize>() -> [&'static str; N] {
    const {
        let id = TypeId::of::<T>();
        assert!(id.fields(0) == N, "N must equal the number of fields");
        let mut names = [""; N];
        let mut i = 0;
        while i < N {
            names[i] = id.field(0, i).name();
            i += 1;
        }
        names
    }
}
```

</details>

So the client and the server **can't disagree** on a param name, and a wrong
reply lists them: `"data": ["key", "value"]`. Add a field to `Create` without
updating the server and the build stops:

```text
error[E0080]: evaluation panicked: N must equal the number of fields
```

In 2023 this took a proc-macro crate.

> [!NOTE]
> The API is brand new ([#146922](https://github.com/rust-lang/rust/issues/146922)),
> so expect it to change between nightlies. Two things this nightly insists on:
> the calls only work inside a `const { }` block, and the type must be
> `'static`, hence `Create<'static>`.

### Linker

On **Linux x86_64**, Rust links with its own `rust-lld` by default since 1.90,
so you get faster links there without changing anything. On **macOS**, rustc
still uses Apple's linker.

<details>
<summary>Trying rust-lld on macOS</summary>

```sh
RUSTFLAGS="-Zunstable-options -Clinker-features=+lld -Clink-self-contained=+linker" cargo build
```

On macOS 26.7 with this nightly, the bundled `rust-lld` crashes when it
starts, so it's not ready here yet. Apple's linker relinks the example server
in about 0.2s anyway.

</details>

## no_std

The protocol side only needs `alloc`. Turn off the default features and
implement `Backend` for your own storage, as [`tests/backend.rs`](tests/backend.rs) does:

```toml
acrudjson = { version = "0.2", default-features = false }
```

| feature | adds |
|---|---|
| `std` *(default)* | `Store`, on redb |
| `tokio` | UDP and TCP servers |

---

## License

MIT or Apache 2.0, at your option.

[redb]: https://www.redb.org
