// crate imports
use crate::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::reactor::reactor::Direction;
use crate::runtime::Executor;

// Mio imports
use mio::Interest;
use mio::Token;
use mio::event::Source;
use mio::net;

use log::info;
/// std imports
use std::future::Future;
use std::io::{self, Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Future representing the operation of reading from a `TcpStream`.
pub struct ReadFuture<'o> {
    io: &'o TcpStream,
    buf: &'o mut [u8],
    token: Token,
}

impl<'o> Future for ReadFuture<'o> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.io.poll_read(cx, self.buf)
    }
}

/// Future representing the operation of writing to a `TcpStream`.
pub struct WriteFuture<'o> {
    io: &'o TcpStream,
    buf: &'o [u8],
    token: Token,
}

impl Future for WriteFuture<'_> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.io.poll_read()
    }
}

/// TCP Socket connected to a listener.
pub struct TcpStream {
    io: mio::net::TcpStream,
    token: Token,
}

/// Impl TcpStream
impl TcpStream {
    /// Create a new TcpStream.
    pub fn new(addr: &str) -> io::Result<TcpStream> {
        let reactor = Executor::get_reactor();

        let address = match addr.parse() {
            Ok(o) => o,
            Err(_e) => return Err(io::Error::new(io::ErrorKind::NotFound, "invalid address")),
        };

        let mut tcp = net::TcpStream::connect(address)?;

        // improve connecting.
        // mio specifies that you should do more checks
        // i shall do them once day.

        let n = reactor.register(&mut tcp, Interest::READABLE | Interest::WRITABLE)?;

        Ok(Self {
            io: tcp,
            token: Token(n),
        })
    }
}

impl AsyncRead for TcpStream {
    /// Read x amount of bytes from this socket.
    /// It's asynchronous woo!!
    fn poll_read<'a>(&self, cx: &mut Context<'_>, buf: &'a mut [u8]) -> Poll<io::Result<usize>> {
        match (&self.io).write(buf) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                info!(
                    "[WRITE] Would block, attaching waker (Write) for Token: {}!",
                    self.token.0
                );
                Executor::get_reactor().attach_waker(cx, self.token, Direction::Write);
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
            Ok(size) => Poll::Ready(Ok(size)),
        }
    }
}

impl AsyncRead for &TcpStream {
    /// Read x amount of bytes from this socket.
    /// It's asynchronous woo!!
    fn poll_read<'a>(&self, cx: &mut Context<'_>, buf: &'a mut [u8]) -> Poll<io::Result<usize>> {
        match (&self.io).write(buf) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                info!(
                    "[WRITE] Would block, attaching waker (Write) for Token: {}!",
                    self.token.0
                );
                Executor::get_reactor().attach_waker(cx, self.token, Direction::Write);
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
            Ok(size) => Poll::Ready(Ok(size)),
        }
    }
}

impl AsyncReadExt for TcpStream {
    /// Read x amount of bytes into `buf` from this socket asychronously.
    /// Returns a `Future` with `io::Result<usize>` as it's Output type.
    async fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        ReadFuture {
            io: self,
            buf,
            token: self.token,
        }
    }
}

impl AsyncReadExt for &TcpStream {
    /// Read x amount of bytes into `buf` from this socket asychronously.
    /// Returns a `Future` with `io::Result<usize>` as it's Output type.
    async fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        ReadFuture {
            io: self,
            buf,
            token: self.token,
        }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write<'w>(&self, cx: &mut Context<'_>, buf: &'w [u8]) -> Poll<io::Result<usize>> {
        match (&self.io).write(buf) {
            Ok(size) => Poll::Ready(Ok(size)),

            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                info!("[READ] Would block, attaching waker (Read)!");

                Executor::get_reactor().attach_waker(cx, self.token, Direction::Write);

                Poll::Pending
            }

            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl AsyncWrite for &TcpStream {
    fn poll_write<'w>(&self, cx: &mut Context<'_>, buf: &'w [u8]) -> Poll<io::Result<usize>> {
        match (&self.io).write(buf) {
            Ok(size) => Poll::Ready(Ok(size)),

            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                info!("[READ] Would block, attaching waker (Read)!");

                Executor::get_reactor().attach_waker(cx, self.token, Direction::Write);

                Poll::Pending
            }

            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl AsyncWriteExt for TcpStream {
    /// Writes x amount of bytes from `buf` to this socket asychronously
    /// Returns a `Future` with `io::Result<usize>` as it's Output type.
    async fn write<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        WriteFuture {
            io: self,
            buf,
            token: self.token,
        }
    }
}

impl AsyncWriteExt for &TcpStream {
    /// Writes x amount of bytes from `buf` to this asychronously
    /// Returns a `Future` with `io::Result<usize>` as it's Output type
    async fn write<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        WriteFuture {
            io: self,
            buf,
            token: self.token,
        }
    }
}

// impl io::Read for &TcpStream {
//     /// Works by calling the underlying `io`'s `read` function.
//     fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
//         let mut io = &self.io;
//         io.read(buf)
//     }
// }

// impl io::Write for &TcpStream {
//     /// Works by calling the underlying `io`'s `write` function.
//     fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
//         let mut io = &self.io;
//         io.write(buf)
//     }

//     /// Works by calling the underlying `io`'s `flush` function.
//     fn flush(&mut self) -> io::Result<()> {
//         let mut io = &self.io;
//         io.flush()
//     }
// }

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
