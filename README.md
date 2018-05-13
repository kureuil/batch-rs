# Batch

[![Crates.io][crates-badge]][crates-url]
[![API Docs][docs-badge]][docs-url]
[![Travis Build Status][travis-badge]][travis-url]
[![Appveyor Build status][appveyor-badge]][appveyor-url]

[crates-badge]: https://img.shields.io/crates/v/batch.svg
[crates-url]: https://crates.io/crates/batch
[docs-badge]: https://docs.rs/batch/badge.svg?version=0.1
[docs-url]: https://docs.rs/batch/0.1
[travis-badge]: https://travis-ci.org/kureuil/batch-rs.svg?branch=master
[travis-url]: https://travis-ci.org/kureuil/batch-rs
[appveyor-badge]: https://ci.appveyor.com/api/projects/status/p8390hfhs1ndmrv9/branch/master?svg=true
[appveyor-url]: https://ci.appveyor.com/project/kureuil/batch-rs/branch/master

A distributed task queue library written in Rust.

Batch allows you to defer work to worker processes, by sending messages to a RabbitMQ broker.
It is a type-safe library that favors safety over performance in order to minimize risk and
avoid mistakes. It leverages the [`futures`] & [`tokio`] crates to provide asynchronous
operations to the user.

[`futures`]: https://crates.io/crates/futures
[`tokio`]: https://crates.io/crates/tokio

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
batch = "0.1"
```

> **Note**: Task serialization depends on [`serde`](https://serde.rs/), so you will have to add it to your project's dependencies as well.

Then add this to your crate root:

```rust
#[macro_use]
extern crate batch;
```

Examples are available on [GitHub][gh-examples] or you can continue and read the [Getting Started][getting-started] guide.

[gh-examples]: https://github.com/kureuil/batch-rs/tree/master/batch/examples
[getting-started]: https://kureuil.github.io/batch-rs/getting-started.html

## Features

* `codegen` *(enabled by default)*: Automatically re-exports the procedurals macros of `batch-codegen` from the `batch` crate.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
