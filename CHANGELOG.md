# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog] and this project adheres to
[Semantic Versioning].

[Keep a Changelog]: http://keepachangelog.com/en/1.0.0/
[Semantic Versioning]: http://semver.org/spec/v2.0.0.html

## [Unreleased]
### Changed
- Batch is now split up in multiple crates, making it a lot more modular.
- **all**: The terminology was changed from "task" to "job": is still conveys the same ideas, but won't collide with futures' terminology of "task".

### Added
- **core**: Added support for job priorities, with 5 levels of granularity: `TRIVIAL`, `LOW`, `NORMAL` (default), `HIGH`, `CRITICAL`.
- **codegen**: The `job` procedural macro to declare jobs.
- **core:** A bunch of traits to allow the use of other message brokers than RabbitMQ.
- **core**: The `Properties` struct, store the metadata associated to a job (timeout, priority, name, etc).
- **rabbitmq**: The `exchanges` and `queues` procedural macros, used to declare RabbitMQ exchanges and queues, which APIs are more strongly typed.

### Fixed
- **rabbitmq:** Exchange name not being used when publishing a task to RabbitMQ.
- **all**: No more `.unwrap()` in documentation examples.
- **all**: Removed last occurences of dangerous `.unwrap()` in the library.

### Removed
- **codegen**: The custom derive for the `Task` trait has been removed, use the `job` procedural macro instead.

## [0.1.1] - 2018-02-22
### Added
- A guide used to document everything that doesn't fit into the API docs.

## 0.1.0 - 2018-02-20
### Added
- Initial release.

[Unreleased]: https://github.com/kureuil/batch-rs/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/kureuil/batch-rs/compare/v0.1.0...v0.1.1
