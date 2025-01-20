use std::future::Future;
use std::io::Result;

pub trait AsyncRead {
    fn async_read<'r>(&'r mut self, buf: &'r mut [u8]) -> impl Future<Output = Result<usize>> + 'r;
}
