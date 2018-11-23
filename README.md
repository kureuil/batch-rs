# Batch [![Crates.io][crates-badge]][crates-url] [![API Docs][docs-badge]][docs-url] [![Travis Build Status][travis-badge]][travis-url] [![Appveyor Build status][appveyor-badge]][appveyor-url]

[crates-badge]: https://img.shields.io/crates/v/batch.svg
[crates-url]: https://crates.io/crates/batch
[docs-badge]: https://docs.rs/batch/badge.svg?version=0.1
[docs-url]: https://docs.rs/batch/0.1
[travis-badge]: https://travis-ci.org/kureuil/batch-rs.svg?branch=master
[travis-url]: https://travis-ci.org/kureuil/batch-rs
[appveyor-badge]: https://ci.appveyor.com/api/projects/status/p8390hfhs1ndmrv9/branch/master?svg=true
[appveyor-url]: https://ci.appveyor.com/project/kureuil/batch-rs/branch/master

A background job library written in Rust.

Batch allows you to defer jobs to worker processes, by sending messages to a broker. It is a type-safe library that favors safety over performance in order to minimize risk and avoid mistakes. It is completely asynchronous and is based on the [`tokio`] runtime.

[`tokio`]: https://crates.io/crates/tokio

## Installation

**Minimum Rust Version:** 1.30

Add this to your `Cargo.toml`:

```toml
[dependencies]
batch = "0.2"
```

Then add this to your crate root:

```rust
extern crate batch;
```

## Batch in action

```rust
extern crate batch;
extern crate batch_rabbitmq;
extern crate tokio;

use batch::job;
use batch_rabbitmq::{queues, Connection};
use std::path::PathBuf;
use tokio::prelude::Future;

queues! {
    Transcoding {
        name = "transcoding",
        bindings = [
            self::transcode,
        ]
    }
}

#[job(name = "batch-example.transcode")]
fn transcode(path: PathBuf) {
    // ...
}

fn main() {
    let fut = Connection::build("amqp://guest:guest@localhost:5672/%2f")
        .declare(Transcoding)
        .connect()
        .and_then(|mut client| {
            let job = transcode("./video.mp4".into());
            Transcoding(job).dispatch(&mut client)
        })
        .map_err(|e| eprintln!("An error occured: {}", e));
    tokio::run(fut);
}
```

More examples are available on [GitHub][gh-examples] and in the [guide][user-guide].

[gh-examples]: https://github.com/kureuil/batch-rs/tree/master/batch/examples
[user-guide]: https://kureuil.github.io/batch-rs/

## Features

* `codegen`: *(enabled by default)*: Enables the use of the `job` procedural macro.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
