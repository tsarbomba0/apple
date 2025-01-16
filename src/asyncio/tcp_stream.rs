use super::io_reactor::IoReactor;
use mio::net;
use mio::Token;
use std::io::Error;
use std::io::Result;

struct TcpStream {
    io: net::TcpStream,
}

impl TcpStream {
    pub fn new(addr: &str) -> Result<TcpStream> {
        let socket_addr = match addr.parse() {
            Err(_e) => return Err(Error::new(std::io::ErrorKind::NotFound, "invalid address")),
            Ok(addr) => addr,
        };
        let io = net::TcpStream::connect(socket_addr)?;

        let reactor = IoReactor::get();

        Ok(TcpStream { io })
    }
}
