# Batch

A distributed task queue library written in Rust using RabbitMQ as a message
broker.

This library allows you to send a task to a RabbitMQ broker, so that a worker
will be able to pull it and execute the associated handler. It leverages the
`futures` and `tokio-core` crates to provide asynchronous I/O operations.

## Goals

* **Safe**: favor safety when possible, minimising risks and mistakes.
* **Extensible**: enable developers to easily add new behaviour to the Query
system.
* **Smart**: allow developers to save time by making the easy obvious, and by
providing sensible defaults.

## Non Goals

* Multiple broker adapters: for the forseeable future RabbitMQ/AMQP will be the
only officially supported message broker
