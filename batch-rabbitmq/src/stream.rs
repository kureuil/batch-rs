use amq_protocol::uri::{AMQPScheme, AMQPUri};
use bytes::{Buf, BufMut};
use failure::Error;
use futures::{Future, IntoFuture, Poll};
use native_tls::TlsConnector;
use std::io::{self, Read, Write};
use std::net;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_reactor::Handle;
use tokio_tcp::TcpStream;
use tokio_tls::TlsConnectorExt;

#[derive(Debug)]
pub enum Stream {
    Raw(::tokio_tcp::TcpStream),
    Tls(::tokio_tls::TlsStream<::tokio_tcp::TcpStream>),
}

impl Stream {
    pub(crate) fn new(uri: AMQPUri) -> impl Future<Item = Self, Error = Error> {
        trace!("Establishing TCP connection");
        net::TcpStream::connect((uri.authority.host.clone().as_ref(), uri.authority.port))
            .map_err(Error::from)
            .into_future()
            .and_then(move |stream| {
                let task = if uri.scheme == AMQPScheme::AMQP {
                    trace!("Wrapping TCP connection into tokio-tcp");
                    let handle = Handle::current();
                    let task = TcpStream::from_std(stream, &handle)
                        .map(Stream::Raw)
                        .map_err(|e| e.into())
                        .into_future();
                    Box::new(task) as Box<Future<Item = Stream, Error = Error> + Send>
                } else {
                    trace!("Wrapping TCP connection into tokio-tls");
                    let host = uri.authority.host.clone();
                    let task = TlsConnector::builder()
                        .map_err(|e| e.into())
                        .into_future()
                        .and_then(|builder| builder.build().map_err(|e| e.into()))
                        .and_then(move |connector| {
                            let handle = Handle::current();
                            TcpStream::from_std(stream, &handle)
                                .map_err(|e| e.into())
                                .into_future()
                                .map(|s| (s, connector))
                        })
                        .and_then(move |(stream, connector)| {
                            connector
                                .connect_async(&host, stream)
                                .map(Stream::Tls)
                                .map_err(|e| e.into())
                        });
                    Box::new(task) as Box<Future<Item = Stream, Error = Error> + Send>
                };
                task
            })
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match *self {
            Stream::Raw(ref mut raw) => raw.read(buf),
            Stream::Tls(ref mut raw) => raw.read(buf),
        }
    }
}

impl AsyncRead for Stream {
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
        match *self {
            Stream::Raw(ref raw) => raw.prepare_uninitialized_buffer(buf),
            Stream::Tls(ref raw) => raw.prepare_uninitialized_buffer(buf),
        }
    }

    fn read_buf<B: BufMut>(&mut self, buf: &mut B) -> Poll<usize, io::Error> {
        match *self {
            Stream::Raw(ref mut raw) => raw.read_buf(buf),
            Stream::Tls(ref mut raw) => raw.read_buf(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match *self {
            Stream::Raw(ref mut raw) => raw.write(buf),
            Stream::Tls(ref mut raw) => raw.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match *self {
            Stream::Raw(ref mut raw) => raw.flush(),
            Stream::Tls(ref mut raw) => raw.flush(),
        }
    }
}

impl AsyncWrite for Stream {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        match *self {
            Stream::Raw(ref mut raw) => raw.shutdown(),
            Stream::Tls(ref mut raw) => raw.shutdown(),
        }
    }

    fn write_buf<B: Buf>(&mut self, buf: &mut B) -> Poll<usize, io::Error> {
        match *self {
            Stream::Raw(ref mut raw) => raw.write_buf(buf),
            Stream::Tls(ref mut raw) => raw.write_buf(buf),
        }
    }
}
