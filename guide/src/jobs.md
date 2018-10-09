# Jobs

## Declaring a job

The simplest way to declare a job is to create a function, and then annotate it with Batch's [`job`] procedural macro:

```rust
extern crate batch;

use batch::job;

#[job(name = "batch-example.say-hello")]
fn say_hello(name: String) {
	println!("Hello {}!", name);
}
```

## Return value of a job

As of today, the [`job`] macro only supports two types of return values:

* either the function return a unit value (the `()` type);
* either the function return a value that implements the [`IntoFuture`] trait (amongst others, it is implemented for both `Future` and `Result`)

The macro not being capable of determining whether the function complies with the above requirements means that if you made a mistake, the compiler will error and the message might not be the clearest, so keep that in mind if you encounter a compile error on one of your job.

## Injecting external values

More often than not, your job will need values that can't or shouldn't be transfered in their message payload. For example, you might need a connection handle to your database, or a mail API client instance, or even the contents of a configuration file. To solve this problem, Batch allow you to mark arguments as *"injected"*. This is done by using the `inject` parameter of the `job` attribute:

```rust
extern crate batch;

use batch::job;
#
# struct UserRepository;
# struct Mailer;

#[job(name = "batch-example.send-hello", inject = [ repository, mailer ])]
fn send_hello(user_id: i32, repository: UserRepository, mailer: Mailer) {
	# drop(user_id);
	# drop(repository);
	# drop(mailer);
	// ...
}
```

In the example above, Batch will only put the `user_id` parameter in the underlying structure, and will fetch the values for the `repository` and the `mailer` values when performing the job. These values are registered on the executor of the job which more often than not will be Batch's own [`Worker`].

## Configuring the job

### Changing the number of retries

By default, a job will be tried 25 times before being declared failed and dropped into a dead-letter queue. This gives you plenty of time to fix your job's implementation. If you wish to change this number, you can do so by using the `retries` parameter of the job procedural macro:

```rust
extern crate batch;

use batch::job;

#[job(name = "batch-example.send-hello", retries = 10)]
fn send_hello(to: String) {
	println!("Hello {}", to);
}

#[job(name = "batch-example.send-goodbye", retries = 50)]
fn send_goodbye(to: String) {
	println!("Goodbye {}", to);
}
```

### Changing the timeout for the job's execution

By default, a job is given 30 minutes to complete. If you wish to change this number, you can do so by using the `timeout` parameter of the `job` procedural macro with the number of seconds the job should be allowed to run:

```rust
extern crate batch;

use batch::job;

// This job will have 1 minute to complete.
#[job(name = "batch-example.send-hello", timeout = "1minute")]
fn send_hello(to: String) {
	println!("Hello {}", to);
}

// This job will have 3 hours and 30 minutes to complete.
#[job(name = "batch-example.send-goodbye", timeout = "3hours 30mins")]
fn send_goodbye(to: String) {
	println!("Goodbye {}", to);
}
```

The timeout parser supports the given suffixes:

* `nsec`, `ns` *-- microseconds*
* `usec`, `us` *-- microseconds*
* `msec`, `ms` *-- milliseconds*
* `seconds`, `second`, `sec`, `s`
* `minutes`, `minute`, `min`, `m`
* `hours`, `hour`, `hr`, `h`
* `days`, `day`, `d`
* `weeks`, `week`, `w`
* `months`, `month`, `M` *-- defined as 30.44 days*
* `years`, `year`, `y` *-- defined as 365.25 days*


### Changing the job priority

By default, a job is assigned the `Normal` priority. If you wish to change this, you can use the `priority` parameter of the `job` procedural macro with one of `trivial`, `low`, `normal`, `high`, `critical`:

```rust
extern crate batch;

use batch::job;

// This job will be assigned the `low` priority.
#[job(name = "batch-example.send-hello", priority = low)]
fn send_hello(to: String) {
	println!("Hello {}", to);
}

// This job will be assigned the `critical` priority.
#[job(name = "batch-example.send-goodbye", priority = critical)]
fn send_goodbye(to: String) {
	println!("Goodbye {}", to);
}
```

---

## Under the hood

Annotating a function with the [`job`] procedural macro will do two things:

* Create a new structure deriving Serde's [`Serialize`] & [`Deserialize`] traits, that stores all of the *non-provided* (see below) function arguments. This implies that all of the arguments must also implement Serde's [`Serialize`] & [`Deserialize`] traits. More importantly, this new structure also implements Batch's [`Job`] trait.
* Change the annotated function to return an instance of the newly defined structure (see previous bullet point), allowing for easy scheduling of the job.

## Get or set the underlying struct of a job

When invoked, [`job`] procedural macro will generate a new structure declaration containing the arguments for the job, and implementing the [`Job`] trait. The name of this structure is the same as the function is comes from. This is made possible by the fact that in Rust, structures and functions don't share the same namespace:

```rust
extern crate batch;

use batch::job;

#[job(name = "batch-example.say-hello")]
fn say_hello(name: String) {
	// ...
}

// Would generate code roughly equivalent to:

# mod generated_do_not_copy_paste_this_please {
struct say_hello {
	name: String
}
# }
```

Hopefully, this simple naming scheme should not conflict with already existing code. If you ever happen to be in this situation, Batch allows you to set the name of the structure that will be generated, by using the `wrapper` parameter:

```rust
extern crate batch;

use batch::job;

#[job(name = "batch-example.say-hello", wrapper = MySuperAwesomeJob)]
fn say_hello(name: String) {
	// ...
}

// Would generate code roughly equivalent to:

# mod generated_do_not_copy_paste_this_please {
struct MySuperAwesomeJob {
	name: String
}
# }
```


[`job`]: https://docs.rs/batch/0.2/index.html
[`IntoFuture`]: https://docs.rs/futures/0.1/futures/future/trait.IntoFuture.html
[`Worker`]: ./worker/index.html
[`Serialize`]: https://docs.serde.rs/serde/trait.Serialize.html
[`Deserialize`]: https://docs.serde.rs/serde/trait.Deserialize.html
[`Job`]: https://docs.rs/batch/0.2/index.html
