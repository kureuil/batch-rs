# Worker

The `Worker` is the most delicate piece of code of the project because of the
choices and trade-offs that have been made. It is a piece software that is
designed to be always running, spawning new tasks as needed & never crashing.

Just like the resque worker, it assumes chaos: each task is executed in its own
process allowing for maximum isolation between tasks and minimum risks of shared
corruption crash. It also enables features such as timeouts that would be
downright impossible to implement with threads for example.

In order to have one process per task, the `Worker` executes itself (the current
executable file) with a special environment variable signaling that it should
read a task payload on the standard input instead of pulling new tasks from the
message broker. This means that it might be complicated to integrate the
`Worker` into an existing binary: the recommended way of using the `Worker` is
by creating a dedicated binary which only goal is pulling tasks & spawning
processes.

See [`Worker` API documentation](https://docs.rs/batch/0.1/batch/struct.Worker.html).
