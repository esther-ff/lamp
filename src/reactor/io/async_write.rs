use crate::io::WriteFut;
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
    };
}
pub trait AsyncWrite {
    fn poll_write<'w>(
        self: Pin<&mut Self>,
        cx: &mut Context<'w>,
        buf: &[u8],
    ) -> Poll<Result<usize>>;
}

impl<T: ?Sized + AsyncWrite + Unpin> AsyncWrite for &mut T {
    write_impl!();
}

impl<T: ?Sized + AsyncWrite + Unpin> AsyncWrite for Box<T> {
    write_impl!();
}

pub trait AsyncWriteExt {
    fn write<'w>(&'w mut self, buf: &'w [u8]) -> WriteFut<'w, Self>;
}
