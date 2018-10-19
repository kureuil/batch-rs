# Concepts

If you're not familiar with Rust or background job libraries, some aspects of batch can feel opaque. Hopefully this page will help clear this up and let you enjoy your projects to the fullest.

## Job

A job is a unit of work that you want to defer to a *Worker* process. For example, imagine you're building a shiny new web application and you want to send the user an email when they sign up. You could send the email synchronously, but this often lead to a poor user experience: maybe the email could take a few seconds to send, making the application feel not very responsive, or maybe the email couldn't be sent because of a spurious network error, or maybe your email provider is down for maintenance. Instead, to guarantee an optimal user experience you will send the job to execute to a message broker which will then be sent to a *Worker* process that will execute it. If the execution fails, the job can be retried either until its retries limit is reached or until it succeeds.

More generally there are two kinds of work you'll want defer:

* Work that has a dependency on an external service, external meaning "on which you have no control, especially in regard to downtime" (e.g: sending emails, etc).
* Work that might take more than a few seconds to complete (e.g: compressing uploaded image files, re-encode video files, etc).

### At-least once delivery & idempotency

One thing that you must keep in mind when you write a job is that it can be executed multimes times, even if it previously succeeded. Writing software is hard, but writing distributed software is even harder: it is possible that because of multiple message broker servers not synchronizing fast enough, or because a connection to a worker gets lost, the same job is given to two different workers. The best you can do to protect yourself is to make your jobs idempotent: meaning your job should *always produce the same output* regardless of how many times it is performed.

### Serialization

In order to send your jobs to message brokers, Batch uses a technic known as serialization: it transforms your in-memory structures with unstable representations into a stable representations that can be used to create a structure that should be identical to the original. Because of this, there's some points you should keep in mind when using batch / declaring jobs:

* **Try to send the minimum amount of information.** Serialization is a linear process, to serialize a structure you have to analyze all of its contents, so the fewer and smaller elements, the faster. For example, instead of sending your `User` struct to your job, send the user's ID and fetch it from the database in the job.
* **Do not send sensible information.** There's no guarantee of privacy / secrecy on message brokers, so you really don't want to put API credentials or user passwords in your job payloads. Instead, you should try to use the [inject] feature of Batch for some of these cases.

## Queue

A queue is the source of jobs for worker processes. It is represented as a never-ending stream of incoming deliveries stating which job should be executed and the environment they should be executed in. In order to consume from a queue, you have to explicitely declare it to your message broker. Instead of using external configuration files, Batch leverages Rust's powerful macro system to ensure your code complies with your expectations (e.g: you shouldn't be able to set a priority on your job if your message broker doesn't support it, with Batch this becomes a compile-time error).

## Worker

A worker is the name given to the process that will subscribe to queues and execute the associated code. It is a long running process that should not crash.

[inject]: jobs.html#injecting-external-values
