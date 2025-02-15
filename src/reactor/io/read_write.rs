use crate::io::{AsyncRead, AsyncWrite};
use mio::Token;
use pin_project_lite::pin_project;
use std::future::Future;
use std::io;
use std::marker::{PhantomPinned, Unpin};
use std::pin::Pin;
use std::task::{Context, Poll};

pin_project! {
    /// Future representing an asynchronous read.
    pub struct ReadFut<'o, IO: ?Sized> {
        io: &'o mut IO,
        buf: &'o mut [u8],
        token: Token,

        #[pin]
        _pin: PhantomPinned,
    }
}

impl<'o, IO: AsyncRead + Unpin + ?Sized> Future for ReadFut<'o, IO> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let pinned = self.project();

        Pin::new(pinned.io).poll_read(cx, pinned.buf)
    }
}

pin_project! {
    /// Future representing an asynchronous write.
    pub struct WriteFut<'o, IO: ?Sized> {
        io: &'o mut IO,
        buf: &'o [u8],
        token: Token,

        #[pin]
        _pin: PhantomPinned,
    }
}

impl<IO: AsyncWrite + Unpin + ?Sized> Future for WriteFut<'_, IO> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let pinned = self.project();

        Pin::new(pinned.io).poll_write(cx, pinned.buf)
    }
}

impl<'w, IO: AsyncWrite + Unpin + ?Sized> WriteFut<'w, IO> {
    pub(crate) fn new(io: &'w mut IO, buf: &'w [u8], token: Token) -> WriteFut<'w, IO> {
        WriteFut {
            io,
            buf,
            token,
            _pin: PhantomPinned,
        }
    }
}

impl<'w, IO: AsyncRead + Unpin + ?Sized> ReadFut<'w, IO> {
    pub(crate) fn new(io: &'w mut IO, buf: &'w mut [u8], token: Token) -> ReadFut<'w, IO> {
        ReadFut {
            io,
            buf,
            token,
            _pin: PhantomPinned,
        }
    }
}
