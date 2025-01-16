use std::future::Future;

pub trait AsyncRead {
    fn read(&self) -> impl Future<Output = ()>;
}

pub trait AsyncWrite {
    async fn write(&self, buf: &[u8]) -> impl Future<Output = usize>;
}
