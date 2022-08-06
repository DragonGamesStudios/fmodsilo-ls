use std::io::{self, Read, Write};

use server::server::{Server, DefaultListener, Sender};

pub struct StdioSender;

impl Sender for StdioSender {
    fn send(&self, message: &String) -> Result<bool, &'static str> {
        print!("{}", message);
        io::stdout().flush().unwrap();

        Ok(true)
    }
}

fn main() {
    let mut server = Server::new(Box::new(DefaultListener::new()), Box::new(StdioSender));

    loop {
        let mut buf: [u8; 1] = [0];
        io::stdin().read_exact(&mut buf).unwrap();

        let mut s = String::new();
        s.push(buf[0] as char);

        match server.push_message(&s) {
            Ok(_) => (),
            Err(v) => for err in v {eprintln!("{err}");}
        };
    }
}
