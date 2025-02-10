use std::future::Future;
use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait AsyncWrite {
    fn poll_write<'w>(&self, cx: &mut Context<'w>, buf: &'w [u8]) -> Poll<Result<usize>>;
}

pub trait AsyncWriteExt {
    async fn write<'w>(&'w mut self, buf: &'w [u8]) -> impl Future<Output = Result<usize>> + 'w;
}
