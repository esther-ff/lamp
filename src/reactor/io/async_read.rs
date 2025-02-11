use crate::io::ReadFut;
use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

macro_rules! read_impl {
    () => {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<Result<usize>> {
            Pin::new(&mut **self).poll_read(cx, buf)
        }
    };
}
pub trait AsyncRead {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8])
    -> Poll<Result<usize>>;
}

impl<T: AsyncRead + Unpin + ?Sized> AsyncRead for &mut T {
    read_impl!();
}

impl<T: AsyncRead + Unpin + ?Sized> AsyncRead for Box<T> {
    read_impl!();
}

pub trait AsyncReadExt {
    fn read<'r>(&'r mut self, buf: &'r mut [u8]) -> ReadFut<'r, Self>;
}
