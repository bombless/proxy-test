use std::net::*;
use std::io::{Read, Write};
const ECHO: &'static [u8] =
    b"HTTP/1.0 200 OK\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\nhello";

struct HttpRequest {
    starter: Vec<u8>,
    headers: Vec<u8>,
}

use std::fmt::{Formatter, Debug, Result as FmtResult};

impl Debug for HttpRequest {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        try!(write!(f, "HttpRequest {{ starter: "));
        for &c in &self.starter {
            try!(write!(f, "{}", (c as char).escape_default().collect::<String>()));
        }
        try!(write!(f, " , headers: "));
        for &c in &self.headers {
            try!(write!(f, "{}", (c as char).escape_default().collect::<String>()));
        }
        write!(f, " }}")
    }
}

fn handle_stream(mut stream: TcpStream) -> Result<HttpRequest, ()> {    
    let mut buf = [0u8; 5];
    const CRLF_CRLF: [u8; 4] = [b'\r', b'\n', b'\r', b'\n'];
    let mut match_count = 0;
    let mut history = Vec::<u8>::new();
    let mut first_line = None;
    'main_loop: loop {
        let n = if let Ok(n) = stream.read(&mut buf) {
            println!("{} bytes read.", n);
            n
        } else {
            continue;
        };

        for (idx, &c) in buf[..n].iter().enumerate() {
            if c == CRLF_CRLF[match_count] {
                match_count += 1;
                if match_count == 4 {
                    if let Some(starter) = first_line {
                        let mut headers = Vec::new();
                        headers.extend(&history);
                        headers.extend(&buf[..idx + 1]);
                        stream.write(ECHO).unwrap();
                        return Ok(HttpRequest {
                            starter: starter,
                            headers: headers,
                        });
                    } else {
                        return Err(());
                    }
                }
                else if match_count == 2 && first_line.is_none() {
                    let mut content = Vec::new();
                    content.extend(&history);
                    content.extend(&buf[..idx + 1]);
                    first_line = Some(content);
                    history.clear();
                    history.extend(&buf[idx + 1..]);
                    continue 'main_loop;
                }
            } else {
                match_count = 0;
            }
        }
        history.extend(&buf[..n]);
    }
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:3386").unwrap();
    for stream in listener.incoming() {
        println!("incoming.");
        println!("{:?}", handle_stream(stream.unwrap()));
        println!("done.");
    }
}
