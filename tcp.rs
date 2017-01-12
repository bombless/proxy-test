use std::net::*;
use std::io::{Read, Write};
const ECHO: &'static [u8] =
    b"HTTP/1.1 200 Connection Established\r\n\r\n";

struct HttpRequest {
    starter: Vec<u8>,
    headers: Vec<u8>,
    stream: TcpStream,
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
                        return Ok(HttpRequest {
                            starter: starter,
                            headers: headers,
                            stream: stream,
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

struct HeaderCatcher {
    match_count: usize,    
}

impl HeaderCatcher {

    pub fn new() -> Self {
        HeaderCatcher { match_count: 0 }
    }

    pub fn handle(&mut self, s: &[u8]) -> Option<usize> {
        const CRLF_CRLF: [u8; 4] = [b'\r', b'\n', b'\r', b'\n'];
        for (idx, &c) in s.iter().enumerate() {
            if c == CRLF_CRLF[self.match_count] {
                self.match_count += 1;
                if self.match_count == 4 {
                    return Some(idx);
                }
            } else {
                self.match_count = 0;
            }
        }
        return None;
    }
}

#[derive(PartialEq, Debug)]
enum HeaderCollector {
    None,
    StartingReturn,
    Start,
    Content(usize),
    WaitColon,
    Coloned,
    Collecting {
        collect: Vec<u8>
    },
    Returned {
        collect: Vec<u8>
    },
    Done {
        collect: Vec<u8>
    },
}

impl HeaderCollector {
    pub fn new() -> Self {
        HeaderCollector::None
    }
    pub fn put(&mut self, c: u8) {
        const CONTENT: &'static [u8] = b"content-length";
        use HeaderCollector::*;
        use std::mem::replace;
        let this = replace(self, None);

        replace(self, match this {
            None if c == b'\r' => StartingReturn,
            StartingReturn if c == b'\n' => Start,
            StartingReturn if c == b'\r' => None,
            Start  => if (c as char).to_lowercase().next() == Some(CONTENT[0] as char) {
                    Content(1)
                } else {
                None
            },
            Content(offset) => if Some(CONTENT[offset] as char) == (c as char).to_lowercase().next() {
                if offset == CONTENT.len() - 1 {
                    WaitColon
                } else {
                    Content(offset + 1)
                }
            } else {
                None
            },
            WaitColon if c == b' ' => WaitColon,
            WaitColon if c == b':' => Coloned,
            Coloned => if c == b' ' { Coloned } else {
                Collecting { collect: vec![c] }
            },
            Collecting { ref collect } if c == b'\r' => {
                Returned { collect: collect.clone() }
            },
            Collecting { mut collect } => {
                collect.push(c);
                Collecting { collect: collect }
            },
            Returned { mut collect } => if c == b'\n'{
                Done { collect: collect }
            } else {
                collect.push(b'\r');
                if c == b'\r' {
                    Returned { collect: collect }
                } else {
                    Collecting { collect: collect }
                }
            },
            _ => None,
        });
    }
}

fn proxy(mut req: HttpRequest) {
    let port = req.port().map(Vec::from);
    println!("port {:?}", port.as_ref().map(|x| escape_bytestring(x)));
    let mut stream = TcpStream::connect((std::str::from_utf8(&port.expect("host not found")).expect("non-UTF8 host"), 80)).expect("connection failed");
    stream.write(req.starter()).unwrap();
    stream.write(req.headers()).unwrap();
    let mut buf = [0; 7];

    let mut length: Option<usize> = None;
    let mut header_catcher = HeaderCatcher::new();
    let mut header_collector = HeaderCollector::new();
    let mut length_count = 0;
    loop {
        if let Ok(n) = stream.read(&mut buf) {
            req.stream.write(&buf[..n]).unwrap();
            if length_count == 0 {
                if let Some(offset) = header_catcher.handle(&buf[..n]) {
                    length_count = n - offset;
                }
            } else {
                length_count += n;
            }

            if let Some(c) = length {
                if length_count >= c {
                    break;
                }
            } else {
                for &c in &buf[..n] {
                    header_collector.put(c);
                    if let HeaderCollector::Done { ref collect } = header_collector {
                        if let Ok(s) = std::str::from_utf8(collect) {
                            length = s.parse().ok();
                            println!("content length {:?}", length)
                        }
                    }
                }
            }
        }
    }
}

impl HttpRequest {

    pub fn port(&self) -> Option<&[u8]> {

        fn find(x: &[u8], c: u8) -> Option<usize> {
            for (idx, &x) in x.iter().enumerate() {
                if x == c {
                    return Some(idx);
                }
            }
            return None;
        }

        match self.host() {
            x @ Some(_) => x,
            _ => match self.path() {
                Some(p) => {
                    let len = "http://".len();
                    let start = &p[len..];
                    if let Some(offset) = find(start, b'/') {
                        Some(&start[..offset])
                    } else {
                        None
                    }
                },
                x => x
            }
        }
    }
    
    pub fn host(&self) -> Option<&[u8]> {

        #[derive(PartialEq, Debug)]
        enum Status {
            None,
            StartingReturn,
            Start,
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
            if matched == Status::Start && !(c == b'H' || c == b'h') {
                matched = Status::None;
            } else if matched == Status::None && c == b'\r' {
                matched = Status::StartingReturn;
            } else if matched == Status::StartingReturn && c == b'\n' {
                matched = Status::Start;
            } else if matched == Status::StartingReturn && c != b'\r' {
                matched = Status::None;
            } else if matched == Status::Start && (c == b'H' || c == b'h') {
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
                return Some(&self.headers[start.expect("null-start") .. idx - 1]);
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

    pub fn method(&self) -> Option<&[u8]> {
        for (idx, &c) in self.starter.iter().enumerate() {
            if (c as char).is_whitespace() {
                return Some(&self.starter[..idx]);
            }
        }
        return None;
    }

    fn starter(&self) -> &[u8] {
        &self.starter
    }

    fn headers(&self) -> &[u8] {
        &self.headers
    }
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:3386").expect("connection failed");
    for stream in listener.incoming() {
        println!("incoming.");
        if let Ok(mut r) = handle_stream(stream.unwrap()) {
            println!("method: {:?}", r.method().map(escape_bytestring));
            println!("path: {:?}", r.path().map(escape_bytestring));
            println!("host: {:?}", r.host().map(escape_bytestring));
            if r.method() == Some(b"CONNECT") {
                r.stream.write(ECHO).expect("write failed");
                println!("replied CONNECT");
            } else {
                println!("going proxy");
                proxy(r);
                println!("done proxy");
            }
        }
        println!("done.");
    }
}
