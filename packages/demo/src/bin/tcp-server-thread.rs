use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0; 512];
    loop {
        match stream.read(&mut buffer) {
            Ok(n) => {
                if n == 0 {
                    break;
                }
                println!("Received: {}", String::from_utf8_lossy(&buffer[..n]));

                let response = b"Hello, client!";
                stream.write_all(response).expect("Failed to write");
                stream.flush().expect("Failed to flush");
            }
            Err(e) => {
                println!("Error reading: {}", e);
                break;
            }
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind");

    println!("Server listening on 127.0.0.1:8080");

    for r in listener.incoming() {
        match r {
            Ok(stream) => {
                thread::spawn(move || handle_client(stream));
            }
            Err(e) => println!("Error accepting: {}", e),
        }
    }
    Ok(())
}
