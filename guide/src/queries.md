# Queries

Queries are the API used when publishing a task to the message broker. A `Query` allows you to overwrite any defaults provided by the `Task` trait and allows you to call any extension registered. A `Query` is manipulated in the same way as a builder: it exposes a fluent interface where you chain method calls, before calling the final method `send`.

You create a query by calling the `job` function, giving it a `Task` instance as parameter, and you send it by calling the `send` method, giving it a reference to your `Client` instance:

```rust
let client = /* your batch Client instance */;
let task = /* your batch Task instance */;
job(task).send(&client);
```

See [`Query` API documentation](https://docs.rs/batch/0.1/batch/struct.Query.html).

