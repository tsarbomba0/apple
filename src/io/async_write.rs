use std::future::Future;
use std::io::Result;

pub trait AsyncWrite {
    fn async_write<'w>(&'w mut self, buf: &'w [u8]) -> impl Future<Output = Result<usize>> + 'w;
}
