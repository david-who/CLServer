// 使用本地库 libecls.rlib
use ecls::server::{Server, Async};

use std::{net::SocketAddr, time::UNIX_EPOCH};
use tokio::time::Duration;
// 0.0.0.0::8080 所有网卡均被监听 (IPAny)
const DEFAULT_SERVER_IPENDPOINT: &'static str = "127.0.0.1:8080";

static mut NOW: u64 = 0;
static mut GLOBAL_BUFFER: [u8; 128] = [0u8; 128];

fn recv(buf: &mut[u8], addr: SocketAddr) -> std::io::Result<()> {
    let msg = String::from_utf8_lossy(&buf[..]);
    println!("📩 Received from {}: {}", addr, msg);

    Ok(())
}

fn send<'a>() -> Option<&'a[u8]> {
    let t0 = unsafe{ NOW };
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH + Duration::new(t0,0))
        .expect("Time went backwards");
    let msg = format!("hello client: {:.2}", now.as_secs_f32());
    let payload = msg.as_bytes();
    let payload_len = payload.len();

    unsafe {
        GLOBAL_BUFFER[..payload_len].copy_from_slice(payload);
    }

    unsafe { Some(&GLOBAL_BUFFER[..payload_len]) }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        NOW = std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    }
    ///////////////////////////////////////////////////////////////////////////////

    let mut server = Server::new(DEFAULT_SERVER_IPENDPOINT,
        "192.168.3.138:8080", 1);
    
    let handle = tokio::spawn(async move {
        server.start_udp_server(Duration::from_millis(1000), recv, send).await;
    });

    ///////////////////////////////////////////////////////////////////////////////

    match handle.await {
        Ok(_) => println!("🛑 Shut down."),
        Err(e) => eprintln!("Task failed: {}", e),
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn test_server() {
        let listener = TcpListener::bind(DEFAULT_SERVER_IPENDPOINT).unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0; 512];
            let read_len = stream.read(&mut buffer).unwrap();
            assert_eq!(b"Hello, server!", &buffer[..read_len]);
            stream.write_all(b"Hello, client!").unwrap();
        });

        let mut stream = TcpStream::connect(DEFAULT_SERVER_IPENDPOINT).unwrap();
        let message = b"Hello, server!";
        stream.write_all(message).unwrap();
        let mut buffer = [0; 512];
        let n = stream.read(&mut buffer).unwrap();
        assert_eq!(b"Hello, client!", &buffer[..n]);

        server.join().unwrap();
    }
}
