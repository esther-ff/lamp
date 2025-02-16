use crate::io::TokenBearer;
use crate::io::{FlushFut, WriteFut};
use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

macro_rules! write_impl {
    () => {
        fn poll_write<'w>(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'w>,
            buf: &[u8],
        ) -> Poll<Result<usize>> {
            Pin::new(&mut **self).poll_write(cx, buf)
        }

        fn poll_flush<'f>(mut self: Pin<&mut Self>, cx: &mut Context<'f>) -> Poll<Result<()>> {
            Pin::new(&mut **self).poll_flush(cx)
        }
    };
}
pub trait AsyncWrite {
    fn poll_write<'w>(
        self: Pin<&mut Self>,
        cx: &mut Context<'w>,
        buf: &[u8],
    ) -> Poll<Result<usize>>;

    fn poll_flush<'f>(self: Pin<&mut Self>, cx: &mut Context<'f>) -> Poll<Result<()>>;
}

impl<T: ?Sized + AsyncWrite + Unpin> AsyncWrite for &mut T {
    write_impl!();
}

impl<T: ?Sized + AsyncWrite + Unpin> AsyncWrite for Box<T> {
    write_impl!();
}

pub trait AsyncWriteExt: AsyncWrite {
    fn write<'w>(&'w mut self, buf: &'w [u8]) -> WriteFut<'w, Self>
    where
        Self: Unpin + AsyncWrite + TokenBearer,
    {
        WriteFut::new(self, buf, self.get_token())
    }

    fn flush<'w>(&'w mut self) -> FlushFut<'w, Self>
    where
        Self: Unpin + AsyncWrite + TokenBearer,
    {
        FlushFut::new(self, self.get_token())
    }
}

impl<Io: AsyncWrite + ?Sized> AsyncWriteExt for Io {}
