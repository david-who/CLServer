use std::net::UdpSocket;
use std::sync::Arc;
use std::thread;

fn main() -> std::io::Result<()> {
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:9001")?);
    println!("Concurrent UDP server on port 9001");

    let mut buf = [0u8; 65535];

    loop {
        let (n, src) = socket.recv_from(&mut buf)?;
        let data: Vec<u8> = buf[..n].to_vec();  // Copy before spawning thread
        let sock = Arc::clone(&socket);

        thread::spawn(move || {
            let msg = std::str::from_utf8(&data).unwrap_or("?");
            println!("Processing from {}: {}", src, msg);
            let reply = format!("Processed: {}", msg);
            if let Err(e) = sock.send_to(reply.as_bytes(), src) {
                eprintln!("Reply error: {}", e);
            }
        });
    }
}