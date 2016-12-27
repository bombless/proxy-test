use std::net::*;
use std::io::{Read, Write};
const ECHO: &'static [u8] =
    b"HTTP/1.0 200 OK\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\nhello";

fn handle_stream(mut stream: TcpStream) {
    println!("incoming.");
    let mut buf = [0u8; 5];
    const CRLF_CRLF: [u8; 4] = [b'\r', b'\n', b'\r', b'\n'];
    let mut match_count = 0;
    let mut history = Vec::<u8>::new();
    let mut first_line = true;
    loop {
        let n = if let Ok(n) = stream.read(&mut buf) {
            println!("{} bytes read.", n);
            n
        } else {
            continue;
        };

        for (idx, &c) in buf[..n].iter().enumerate() {
            if c == CRLF_CRLF[match_count] {
                match_count += 1;
                if match_count == 4 { break; }
                else if match_count == 2 && first_line {
                    first_line = false;
                    println!("history length {} idx {}, now:", history.len(), idx);
                    fn mapping(&x: &u8) -> String { (x as char).escape_default().collect() };
                    print!("{}", history.iter().map(&mapping).collect::<String>());
                    println!("{}", buf[..idx + 1].iter().map(mapping).collect::<String>());
                }
            } else {
                match_count = 0;
            }
        }
        if first_line {
            history.extend(&buf[..n]);
        }
        if match_count > 0 {
            println!("{} matched", match_count);
        }
        if match_count == 4 {
            stream.write(ECHO).unwrap();
            break;
        }
    }
    println!("done.");
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:3386").unwrap();
    for stream in listener.incoming() {
        handle_stream(stream.unwrap());
    }
}
