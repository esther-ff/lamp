mod iosource;

/// TcpStream struct.
pub use tcp_stream::TcpStream;
mod tcp_stream;

/// Trait for asychronous reads.
pub mod async_read;
pub use async_read::AsyncRead;
pub use async_read::AsyncReadExt;

/// Trait for asynchronous writes.
pub mod async_write;
pub use async_write::AsyncWrite;
pub use async_write::AsyncWriteExt;

mod read_write;
pub(crate) use read_write::{ReadFut, WriteFut};
