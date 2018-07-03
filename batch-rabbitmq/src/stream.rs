use std::io::{self, Read, Write};

use bytes::{Buf, BufMut};
use futures::Poll;
use tokio_io::{AsyncRead, AsyncWrite};

#[derive(Debug)]
pub enum Stream {
    Raw(::tokio_tcp::TcpStream),
    Tls(::tokio_tls::TlsStream<::tokio_tcp::TcpStream>),
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
