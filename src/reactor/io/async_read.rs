use std::future::Future;
use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait AsyncRead {
    fn poll_read<'r>(&self, cx: &mut Context<'r>, buf: &'r mut [u8]) -> Poll<Result<usize>>
}

pub trait AsyncReadExt {
    async fn read<'r>(&'r mut self, buf: &'r mut [u8]) -> impl Future<Output = Result<usize>> + 'r;
}
