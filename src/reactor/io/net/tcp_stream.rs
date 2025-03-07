use crate::io::TokenBearer;
use crate::io::{AsyncRead, AsyncWrite};
use crate::reactor::reactor::Direction;
use crate::runtime::{Executor, ExecutorHandle};

use mio::Interest;
use mio::Token;
use mio::event::Source;
use mio::net;

//use log::info;

use std::io::{self, Read, Write};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

macro_rules! handle_async_read {
    ($io: expr, $buf: expr, $cx: expr, $token: expr, $handle: expr) => {
        match (&$io).read($buf) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                $handle.reactor_fn(|r| r.attach_waker($cx, $token, Direction::Read));
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
            Ok(size) => Poll::Ready(Ok(size)),
        }
    };
}

macro_rules! handle_async_write {
    ($io: expr, $buf: expr, $cx: expr, $token: expr, $handle: expr) => {
        match (&$io).write($buf) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                $handle.reactor_fn(|r| r.attach_waker($cx, $token, Direction::Write));
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
            Ok(size) => Poll::Ready(Ok(size)),
        }
    };
}

macro_rules! handle_async_flush {
    ($io: expr, $cx: expr, $token: expr, $handle: expr) => {
        match (&$io).flush() {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                $handle.reactor_fn(|r| r.attach_waker($cx, $token, Direction::Write));
                Poll::Pending
            }

            Err(e) => Poll::Ready(Err(e)),
            Ok(_) => Poll::Ready(Ok(())),
        }
    };
}

/// TCP Socket connected to a listener.
pub struct TcpStream {
    io: mio::net::TcpStream,
    handle: Arc<ExecutorHandle>,
    pub(crate) token: Token,
}

/// Impl TcpStream
impl TcpStream {
    /// Create a new TcpStream.
    pub fn new(addr: &str) -> io::Result<TcpStream> {
        let address = match addr.parse() {
            Ok(o) => o,
            Err(_e) => return Err(io::Error::new(io::ErrorKind::NotFound, "invalid address")),
        };

        let mut tcp = net::TcpStream::connect(address)?;

        // improve connecting.
        // mio specifies that you should do more checks
        // i shall do them once day.

        let handle = Executor::get();

        let result =
            handle.reactor_fn(|r| r.register(&mut tcp, Interest::READABLE | Interest::WRITABLE));

        Ok(Self {
            io: tcp,
            handle,
            token: Token(result?),
        })
    }

    pub fn from_std(tcp: std::net::TcpStream) -> io::Result<Self> {
        tcp.set_nonblocking(true)?;
        let mut sock = mio::net::TcpStream::from_std(tcp);
        let handle = Executor::get();
        let result =
            handle.reactor_fn(|r| r.register(&mut sock, Interest::READABLE | Interest::WRITABLE));

        Ok(Self {
            io: sock,
            handle,
            token: Token(result?),
        })
    }
}

impl TokenBearer for TcpStream {
    fn get_token(&self) -> Token {
        self.token
    }
}

impl AsyncRead for TcpStream {
    /// Read x amount of bytes from this socket.
    /// It's asynchronous woo!!
    fn poll_read<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &'a mut [u8],
    ) -> Poll<io::Result<usize>> {
        handle_async_read!(self.io, buf, cx, self.token, self.handle)
    }
}

impl AsyncRead for &TcpStream {
    /// Read x amount of bytes from this socket.
    /// It's asynchronous woo!!
    fn poll_read<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &'a mut [u8],
    ) -> Poll<io::Result<usize>> {
        handle_async_read!(self.io, buf, cx, self.token, self.handle)
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write<'w>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &'w [u8],
    ) -> Poll<io::Result<usize>> {
        handle_async_write!(self.io, buf, cx, self.token, self.handle)
    }

    fn poll_flush<'f>(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        handle_async_flush!(self.io, cx, self.token, self.handle)
    }
}

impl AsyncWrite for &TcpStream {
    fn poll_write<'w>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &'w [u8],
    ) -> Poll<io::Result<usize>> {
        handle_async_write!(self.io, buf, cx, self.token, self.handle)
    }

    fn poll_flush<'f>(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        handle_async_flush!(self.io, cx, self.token, self.handle)
    }
}

impl Source for TcpStream {
    fn register(
        &mut self,
        reg: &mio::Registry,
        token: Token,
        intr: mio::Interest,
    ) -> io::Result<()> {
        self.io.register(reg, token, intr)
    }

    fn reregister(
        &mut self,
        reg: &mio::Registry,
        token: Token,
        intr: mio::Interest,
    ) -> io::Result<()> {
        self.io.reregister(reg, token, intr)
    }

    fn deregister(&mut self, registry: &mio::Registry) -> io::Result<()> {
        self.io.deregister(registry)
    }
}
