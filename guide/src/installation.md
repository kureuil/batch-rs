# Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
batch = "0.1"
```

> **Note**: Task serialization depends on [`serde`], so you will have to add it
to your project's dependencies as well.

Then add this to your crate root:

```rust
#[macro_use]
extern crate batch;
```

Examples are available on [GitHub][examples] or you can continue and read the
Getting Started guide.

[`serde`]: https://serde.rs
[examples]: https://github.com/kureuil/batch-rs/tree/master/batch/examples

