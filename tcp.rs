use std::net::*;
use std::io::{Read, Write};
const ECHO: &'static [u8] =
    b"HTTP/1.0 200 OK\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\nhello";
fn main() {
    let listener = TcpListener::bind("0.0.0.0:3386").unwrap();
    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            println!("incoming.");
            let mut buf = [0u8; 16];
            const CRLF_CRLF: [u8; 4] = [b'\r', b'\n', b'\r', b'\n'];
            let mut match_count = 0;
            let mut history = Vec::<u8>::new();
            let mut first_line = true;
            loop {
                if let Ok(n) = stream.read(&mut buf) {
                    println!("{} bytes read.", n);
                } else {
                    continue;
                }
                
                for (idx, &c) in buf.iter().enumerate() {
                    if c == CRLF_CRLF[match_count] {
                        match_count += 1;
                        if match_count == 4 { break; }
                        else if match_count == 2 && first_line {
                            first_line = false;
                            println!("history length {} idx {}, now:", history.len(), idx);
                            let mapping = |&x| (x as char).escape_default().collect::<String>();
                            println!("{}", history.iter().map(&mapping).collect::<String>());
                            println!("{}", buf[..idx + 1].iter().map(mapping).collect::<String>());
                        }
                    } else {
                        match_count = 0;
                    }
                }
                history.extend(&buf);
                if match_count > 0 {
                    println!("{} matched", match_count);
                }
                if match_count == 4 {
                    stream.write(ECHO).unwrap();
                    break;
                }
            }
            println!("done.");
        } else {
            println!("nothing.");
        }        
    }
}
