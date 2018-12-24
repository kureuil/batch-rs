use std::fmt;

use crate::consumer::Consumer;

/// The connection to the Redis server.
///
/// This is the main struct of this crate.
#[derive(Clone)]
pub struct Connection {
    inner: redis::r#async::SharedConnection,
}

impl Connection {
    /// Open a connection to the server provided in parameter.
    ///
    /// The server credentials can be submitted using any type implementing the
    /// `redis::IntoConnectionInfo` trait, though the two most common forms of
    /// credentials will be `&str` and `redis::ConnectionInfo`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use batch_redis::Connection;
    ///
    /// let fut = batch_redis::Connection::open("redis://localhost:6379/0");
    /// ```
    pub fn open<P>(params: P) -> OpenFuture<P>
    where
        P: redis::IntoConnectionInfo,
    {
        OpenFuture {
            state: OpenFutureState::Initial {
                params: Some(params),
            },
        }
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Connection").finish()
    }
}

impl batch::Client for Connection {
    type SendFuture = Box<futures::Future<Item = (), Error = failure::Error> + Send>;

    fn send(&mut self, _dispatch: batch::Dispatch) -> Self::SendFuture {
        unimplemented!();
    }

    type Consumer = Consumer;

    type ToConsumerFuture =
        Box<futures::Future<Item = Self::Consumer, Error = failure::Error> + Send>;

    fn to_consumer(
        &mut self,
        _queues: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self::ToConsumerFuture {
        unimplemented!();
    }
}

/// The future returned when opening a connection to a Redis server.
pub struct OpenFuture<P> {
    state: OpenFutureState<P>,
}

enum OpenFutureState<P> {
    Initial {
        params: Option<P>,
    },
    Connecting {
        fut: Box<
            futures::Future<Item = redis::r#async::Connection, Error = redis::RedisError> + Send,
        >,
    },
    Sharing {
        fut: Box<
            futures::Future<Item = redis::r#async::SharedConnection, Error = redis::RedisError>
                + Send,
        >,
    },
}

impl<P> fmt::Debug for OpenFuture<P>
where
    P: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("OpenFuture").finish()
    }
}

impl<P> futures::Future for OpenFuture<P>
where
    P: redis::IntoConnectionInfo,
{
    type Item = Connection;

    type Error = OpenError;

    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        match self.state {
            OpenFutureState::Initial { ref mut params } => {
                let params = params.take().expect("reused OpenFuture");
                let client = redis::Client::open(params).map_err(ErrorKind::ConnectionInfo)?;
                self.state = OpenFutureState::Connecting {
                    fut: client.get_async_connection(),
                };
                self.poll()
            }
            OpenFutureState::Connecting { ref mut fut } => match fut.poll() {
                Ok(futures::Async::Ready(conn)) => {
                    self.state = OpenFutureState::Sharing {
                        fut: redis::r#async::SharedConnection::new(conn),
                    };
                    self.poll()
                }
                Ok(futures::Async::NotReady) => Ok(futures::Async::NotReady),
                Err(e) => Err(ErrorKind::Connect(e).into()),
            },
            OpenFutureState::Sharing { ref mut fut } => match fut.poll() {
                Ok(futures::Async::Ready(shared)) => {
                    Ok(futures::Async::Ready(Connection { inner: shared }))
                }
                Ok(futures::Async::NotReady) => Ok(futures::Async::NotReady),
                Err(e) => Err(ErrorKind::Share(e).into()),
            },
        }
    }
}

/// Errors that can be encountered when open a connection to a Redis server.
#[derive(Debug)]
pub struct OpenError(failure::Context<ErrorKind>);

#[derive(Debug, failure::Fail)]
enum ErrorKind {
    #[fail(display = "Couldn't parse redis credentials")]
    ConnectionInfo(#[fail(cause)] redis::RedisError),
    #[fail(display = "Couldn't connect to the redis server")]
    Connect(#[fail(cause)] redis::RedisError),
    #[fail(display = "Couldn't created shared connection from unique connection")]
    Share(#[fail(cause)] redis::RedisError),
}

impl failure::Fail for OpenError {
    fn cause(&self) -> Option<&failure::Fail> {
        self.0.cause()
    }

    fn backtrace(&self) -> Option<&failure::Backtrace> {
        self.0.backtrace()
    }
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<failure::Context<ErrorKind>> for OpenError {
    fn from(context: failure::Context<ErrorKind>) -> OpenError {
        OpenError(context)
    }
}

impl From<ErrorKind> for OpenError {
    fn from(kind: ErrorKind) -> OpenError {
        OpenError(failure::Context::new(kind))
    }
}
