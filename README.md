# Batch

A distributed task queue library written in Rust using RabbitMQ as a message broker.

This library allows you to send a task to a RabbitMQ broker, so that a worker will be able
to pull it and execute the associated handler. It leverages the `futures` and `tokio-core`
crates to provide asynchronous I/O operations.

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

Examples are available on [GitHub](https://github.com/kureuil/batch-rs/tree/master/batch/examples) or you can continue and read the [Getting Started](https://kureuil.github.io/batch-rs/getting-started.html) guide.

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
