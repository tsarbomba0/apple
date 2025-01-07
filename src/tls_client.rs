use mio::net::TcpStream;
use rustls::{ClientConfig, ClientConnection, RootCertStore};
use rustls_pki_types::ServerName;
use std::io;
use std::io::{ErrorKind, Read, Result, Write};
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};

use webpki_roots::TLS_SERVER_ROOTS;

struct Inner {
    sock: Mutex<TcpStream>,
    conn: Mutex<ClientConnection>,
}

#[derive(Clone)]
pub struct TlsClient {
    token: mio::Token,
    inner: Arc<Inner>,
}

impl TlsClient {
    // private functions
    fn c_read(&mut self) {
        let inner = Arc::clone(&self.inner);
        let mut sock = inner.sock.lock().unwrap();
        let mut conn = inner.conn.lock().unwrap();

        match conn.read_tls(&mut *sock) {
            Err(ref e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    return;
                }
                println!("TLS read error!");
                //let bool = self.inner.closing.get_mut();
                //bool = true;
                return;
            }
            Ok(0) => {
                println!("TLS Eof!");
                //self.closing = true;
                return;
            }
            Ok(_) => {}
        };

        let state = match conn.process_new_packets() {
            Err(ref e) => {
                //self.closing = true;
                println!("TLS PROTOCOL ERROR: {}", e);
                return;
            }
            Ok(i) => i,
        };

        if state.plaintext_bytes_to_read() > 0 {
            let mut buf = vec![0u8; state.plaintext_bytes_to_read()];

            conn.reader().read_exact(&mut buf).expect("Reading failed!");

            io::stdout()
                .write_all(&buf)
                .expect("Writing to stdout failed!");
        }
    }
    fn c_write(&mut self) {
        let inner = Arc::clone(&self.inner);
        let mut sock = inner.sock.lock().unwrap();
        let mut conn = inner.conn.lock().unwrap();
        conn.write_tls(&mut *sock).expect("TLS write failed");
    }
    fn interest(&self) -> mio::Interest {
        let conn = self.inner.conn.lock().unwrap();
        let (r, w) = (conn.wants_read(), conn.wants_write());

        if r && w {
            mio::Interest::READABLE | mio::Interest::WRITABLE
        } else if w {
            mio::Interest::WRITABLE
        } else {
            mio::Interest::READABLE
        }
    }
    pub fn register(&mut self, registry: &mio::Registry) {
        let int = self.interest();
        let inner = Arc::clone(&self.inner);
        let mut sock = inner.sock.lock().unwrap();

        registry
            .register(&mut *sock, self.token, int)
            .expect("Registering failed!");
    }
    pub fn reregister(&mut self, registry: &mio::Registry) {
        let int = self.interest();
        let inner = Arc::clone(&self.inner);
        let mut sock = inner.sock.lock().unwrap();

        registry
            .reregister(&mut *sock, self.token, int)
            .expect("Reregistering failed!");
    }
    pub fn ready(&mut self, ev: &mio::event::Event) {
        assert_eq!(ev.token(), self.token);

        if ev.is_readable() {
            self.c_read();
        }
        if ev.is_writable() {
            self.c_write();
        }
        if ev.is_read_closed() || ev.is_write_closed() {
            println!("Closed!");
            //std::process::exit(if self.clean_close { 0 } else { 1 })
        }
    }

    pub fn new(host: &str, port: u16, token: mio::Token) -> Result<TlsClient> {
        let addr = (host, port)
            .to_socket_addrs()
            .expect("Invalid hostname!")
            .next()
            .unwrap();
        let sock = Mutex::new(TcpStream::connect(addr)?);

        let root_cert = RootCertStore {
            roots: TLS_SERVER_ROOTS.into(),
        };

        let cfg = Arc::new(
            ClientConfig::builder()
                .with_root_certificates(root_cert)
                .with_no_client_auth(),
        );

        let server_name: ServerName = host.to_owned().try_into().expect("Invalid hostname");
        let conn = Mutex::new(ClientConnection::new(cfg, server_name).unwrap());
        let inner = Arc::new(Inner { sock, conn });

        Ok(Self { token, inner })
    }
    pub fn get_token(&self) -> &mio::Token {
        &self.token
    }
}

impl io::Write for TlsClient {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut i = self.inner.conn.lock().unwrap();
        i.writer().write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut i = self.inner.conn.lock().unwrap();
        i.writer().flush()
    }
}

impl io::Read for TlsClient {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let mut i = self.inner.conn.lock().unwrap();
        i.reader().read(bytes)
    }
}
