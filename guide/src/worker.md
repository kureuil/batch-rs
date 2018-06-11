# Worker

The `Worker` is the most delicate piece of code of the project because of the
choices and trade-offs that have been made. It is a piece software that is
designed to be always running, spawning new jobs as needed & never crashing.

Just like the resque worker, it assumes chaos: each job is executed in its own
process allowing for maximum isolation between jobs and minimum risks of shared
corruption crash. It also enables features such as timeouts that would be
downright impossible to implement with threads for example.

In order to have one process per job, the `Worker` executes itself (the current
executable file) with a special environment variable signaling that it should
read a job payload on the standard input instead of pulling new jobs from the
message broker. This means that it might be complicated to integrate the
`Worker` into an existing binary: the recommended way of using the `Worker` is
by creating a dedicated binary which only goal is pulling jobs & spawning
processes.

By default, the `Worker` will process as many jobs in parallel as there are
logical cores on the system. You can tweak this number when creating a
`Worker` using the [`WorkerBuilder::parallelism`] method.

See [`Worker` API documentation](https://docs.rs/batch/0.1/batch/struct.Worker.html).

[`WorkerBuilder::parallelism`]: https://docs.rs/batch/0.1/batch/struct.WorkerBuilder.html#method.parallelism