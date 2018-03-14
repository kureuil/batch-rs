# Queries

Queries are the API used when publishing a task to the message broker. A `Query`
allows you to overwrite any defaults provided by the `Task` trait and allows you
to call any extension registered. A `Query` is manipulated in the same way as a
builder: it exposes a fluent interface where you chain method calls, before
calling the final method `send`.

You create a query by calling the `job` function, giving it a `Task` instance as
parameter, and you send it by calling the `send` method, giving it a reference
to your `Client` instance:

```rust
let client = /* your batch Client instance */;
let task = /* your batch Task instance */;
job(task).send(&client);
```

See [`Query` API documentation](https://docs.rs/batch/0.1/batch/struct.Query.html).

## Extending `Query`

By defining an [extension trait], you can add new methods to the [`Query`] type.
In order to make them useful, `batch` provides several methods that are meant to
be called from your extension traits:

- [`properties`][Query::properties]
- [`properties_mut`][Query::properties_mut]
- [`options`][Query::options]
- [`options_mut`][Query::options_mut]

[`Query`]: https://docs.rs/batch/0.1/batch/struct.Query.html
[Query::properties]: https://docs.rs/batch/0.1/batch/struct.Query.html#method.properties
[Query::properties_mut]: https://docs.rs/batch/0.1/batch/struct.Query.html#method.properties_mut
[Query::options]: https://docs.rs/batch/0.1/batch/struct.Query.html#method.options
[Query::options_mut]: https://docs.rs/batch/0.1/batch/struct.Query.html#method.options_mut

## Extending `ExchangeBuilder`

By defining an [extension trait], you can add new methods to the
[`ExchangeBuilder`] type. In order to make them useful, `batch` provides several
methods that are meant to be called from your extension traits:

- [`options`][ExchangeBuilder::options]
- [`options_mut`][ExchangeBuilder::options_mut]
- [`arguments`][ExchangeBuilder::arguments]
- [`arguments_mut`][ExchangeBuilder::arguments_mut]

[`ExchangeBuilder`]: https://docs.rs/batch/0.1/batch/struct.ExchangeBuilder.html
[ExchangeBuilder::options]: https://docs.rs/batch/0.1/batch/struct.ExchangeBuilder.html#method.options
[ExchangeBuilder::options_mut]: https://docs.rs/batch/0.1/batch/struct.ExchangeBuilder.html#method.options_mut
[ExchangeBuilder::arguments]: https://docs.rs/batch/0.1/batch/struct.ExchangeBuilder.html#method.arguments
[ExchangeBuilder::arguments_mut]: https://docs.rs/batch/0.1/batch/struct.ExchangeBuilder.html#method.arguments_mut

## Extending `Queue`

By defining an [extension trait], you can add new methods to the
[`QueueBuilder`] type. In order to make them useful, `batch` provides several
methods that are meant to be called from your extension traits:

- [`options`][QueueBuilder::options]
- [`options_mut`][QueueBuilder::options_mut]
- [`arguments`][QueueBuilder::arguments]
- [`arguments_mut`][QueueBuilder::arguments_mut]

[`QueueBuilder`]: https://docs.rs/batch/0.1/batch/struct.QueueBuilder.html
[QueueBuilder::options]: https://docs.rs/batch/0.1/batch/struct.QueueBuilder.html#method.options
[QueueBuilder::options_mut]: https://docs.rs/batch/0.1/batch/struct.QueueBuilder.html#method.options_mut
[QueueBuilder::arguments]: https://docs.rs/batch/0.1/batch/struct.QueueBuilder.html#method.arguments
[QueueBuilder::arguments_mut]: https://docs.rs/batch/0.1/batch/struct.QueueBuilder.html#method.arguments_mut


[extension trait]: https://github.com/rust-lang/rfcs/blob/master/text/0445-extension-trait-conventions.md
