use std::net::*;
use std::io::{Read, Write};
const ECHO: &'static [u8] =
    b"HTTP/1.0 200 OK\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\nhello";

struct HttpRequest {
    starter: Vec<u8>,
    headers: Vec<u8>,
}

use std::fmt::{Formatter, Debug, Result as FmtResult};

fn escape_bytestring(s: &[u8]) -> String {
    s.iter().map(|&x| (x as char).escape_default().collect::<String>()).collect()
}

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

impl HttpRequest {
    
    pub fn host(&self) -> Option<&[u8]> {

        #[derive(PartialEq, Debug)]
        enum Status {
            None,
            First,
            Second,
            Third,
            Fourth,
            Coloned,
            Collecting,
            Returned,

        }
        let mut start = None;
        let mut matched = Status::None;
        for (idx, &c) in self.headers.iter().enumerate() {
            println!("{:?}", matched);
            if matched == Status::None && (c == b'H' || c == b'h') {
                matched = Status::First;
            } else if matched == Status::First && (c == b'O' || c == b'o') {
                matched = Status::Second;
            } else if matched == Status::Second && (c == b'S' || c == b's') {
                matched = Status::Third;
            } else if matched == Status::Third && (c == b'T' || c == b't') {
                matched = Status::Fourth;
            } else if matched == Status::Fourth && c == b':' {
                matched = Status::Coloned;
            } else if matched == Status::Coloned && c != b' ' {
                start = Some(idx);
                matched = Status::Collecting;
            } else if matched == Status::Collecting && c == b'\r' {
                matched = Status::Returned;
            } else if matched == Status::Returned && c == b'\n' {
                return Some(&self.headers[start.unwrap() .. idx - 1]);
            } else if !(c == b' ' && (matched == Status::Fourth || matched == Status::Coloned)) && matched != Status::Collecting && matched != Status::Returned {
                matched = Status::None;
            } else if matched == Status::Returned && c != b'\r' {
                matched = Status::Collecting;
            }
        }
        None
    }

    pub fn path(&self) -> Option<&[u8]> {
        let mut gap = None;
        for (idx, &c) in self.starter.iter().enumerate() {
            if (c as char).is_whitespace() {
                if let Some(s) = gap {
                    return Some(&self.starter[s..idx]);
                }
                gap = Some(idx + 1);
            }
        }
        return None;
    }
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:3386").unwrap();
    for stream in listener.incoming() {
        println!("incoming.");
        if let Ok(r) = handle_stream(stream.unwrap()) {
            println!("path: {:?}", r.path().map(escape_bytestring));
            println!("host: {:?}", r.host().map(escape_bytestring));
        }
        println!("done.");
    }
}
