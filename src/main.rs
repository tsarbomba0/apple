use mio::{Events, Poll};
mod reactor;
mod tls_client;

use async_lock::OnceCell;
use mio::Token;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tls_client::TlsClient;

const CLIENT: Token = Token(0);

fn main_loop() {
    loop {
        let r = Reactor::get();

        println!("We workin!");
        let mut r_poll = r.poll.lock().unwrap();
        let mut r_events = r.events.lock().unwrap();
        let mut r_reg = r.registry.lock().unwrap();

        r_poll.poll(&mut r_events, None).unwrap();

        for ev in r_events.iter() {
            let conn = r_reg.get_mut(&ev.token());
            println!("{:?}", &ev.token());
            match conn {
                Some(conn) => {
                    conn.ready(ev);
                    conn.reregister(r_poll.registry())
                }
                None => println!("uhoh!"),
            }
        }
    }
}

struct Reactor {
    poll: Mutex<Poll>,
    events: Mutex<Events>,
    registry: Mutex<HashMap<Token, TlsClient>>,
}

impl Reactor {
    fn get() -> &'static Reactor {
        static REACTOR: OnceCell<Reactor> = OnceCell::new();

        REACTOR.get_or_init_blocking(|| {
            thread::Builder::new()
                .name("helper-reactor-thread".to_string())
                .spawn(move || main_loop())
                .expect("Can't get helper thread!");

            Reactor {
                poll: Mutex::new(Poll::new().unwrap()),
                events: Mutex::new(Events::with_capacity(64)),
                registry: Mutex::new(HashMap::new()),
            }
        })
    }
    fn r_register(&self, thingy: &mut TlsClient) {
        let poll = self.poll.lock().unwrap();
        let registry = poll.registry();
        thingy.register(registry);
        drop(poll);

        let mut reg = self.registry.lock().unwrap();
        reg.insert(*thingy.get_token(), thingy.clone());
    }
}

fn main() {
    println!("Hello, world!");

    let mut stream =
        TlsClient::new("www.rust-lang.org", 443, CLIENT).expect("Failed to create client!");

    let message = concat!(
        "GET / HTTP/1.1\r\n",
        "Host: www.rust-lang.org\r\n",
        //"Connection: close\r\n",
        "Accept: */*\r\n",
        "User-Agent: testing/0.0.1\r\n",
        "\r\n"
    )
    .as_bytes();

    stream.write_all(message).unwrap();
    stream.write_all(message).unwrap();
    stream.write_all(message).unwrap();
    stream.write_all(message).unwrap();
    stream.write_all(message).unwrap();
    // let mut poll = Poll::new().unwrap();
    // let mut evns = Events::with_capacity(64);
    let r = Reactor::get();

    r.r_register(&mut stream);

    loop {
        std::thread::sleep(Duration::from_secs(1))
    }
}
