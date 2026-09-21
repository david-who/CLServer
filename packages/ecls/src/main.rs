#[path ="modules/server.rs"]
pub mod server;

#[path ="modules/client.rs"]
pub mod client;

#[path ="modules/ecpm.rs"]
pub mod ecpm;

#[path ="modules/ecshm.rs"]
pub mod ecshm;

//use ecls::ecpm::{EcPmShm, EcPM_Buffer_T};
use server::{Server, Async};

use std::net::SocketAddr;
use tokio::time::Duration;
//use std::sync::Arc; // Atomic Operations for thread-safe shared state
//use std::sync::atomic::{AtomicBool, Ordering}; // Atomic Types


// 0.0.0.0::8080 所有网卡均被监听 (IPAny)
const DEFAULT_SERVER_IPENDPOINT: &'static str = "127.0.0.1:8080";

static mut GLOBAL_BUFFER: [u8; 128] = [0u8; 128];

fn recv(buf: &mut[u8], addr: SocketAddr) -> std::io::Result<()> {
    let msg = String::from_utf8_lossy(&buf[..]);
    println!("📩 Received from {}: {}", addr, msg);

    Ok(())
}

fn send<'a>() -> Option<&'a[u8]> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH + Duration::new(1789029003,0))
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
/* CTRL+C: ARC
    let running = Arc::new(AtomicBool::new(true));
    let shutdown_flag = Arc::clone(&running);

    ctrlc::set_handler(move || {
        shutdown_flag.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");
*/

    ///////////////////////////////////////////////////////////////////////////////

    let mut server = Server::new(DEFAULT_SERVER_IPENDPOINT,
        "192.168.3.138:8080", 1);
    
    let handle = tokio::spawn(async move {
        server.start_udp_server(Duration::from_millis(1000), recv, send).await;
    });

    ///////////////////////////////////////////////////////////////////////////////
/* CTRL+C:
    while running.load(Ordering::SeqCst) {
        sleep(Duration::from_millis(1000)).await;
    }
*/
    match handle.await {
        Ok(_) => println!("🛑 Shut down."),
        Err(e) => eprintln!("Task failed: {}", e),
    }

    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpStream;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn handle_tcp_client(mut stream: TcpStream) -> std::io::Result<()> {
        let mut buffer = [0; 512];
        loop {
            match stream.read(&mut buffer).await {
                Ok(n) => {
                    if n == 0 { // client closed connection
                        break;
                    }
                    assert_eq!(b"Hello, server!\n", &buffer[..n], "OK Server.");

                    let response = b"Hello, client!";
                    stream.write_all(response).await.expect("Failed to write");
                    stream.flush().await.expect("Failed to flush");
                }
                Err(_) => { // read error
                    assert!(false, "Error reading:");
                    break;
                }
            }
        }
        Ok(())
    }


    #[tokio::test]
    async fn server_tcp_test() {
        let mut server1 = Server::new(DEFAULT_SERVER_IPENDPOINT,
            DEFAULT_SERVER_IPENDPOINT, 1);

        // TCP Server
        tokio::spawn(async move {
            server1.start_tcp_server_full(handle_tcp_client).await;
        });
        // Give server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        // TCP Client
        {
            let mut stream = TcpStream::connect(DEFAULT_SERVER_IPENDPOINT).await.unwrap();
            let message = b"Hello, server!\n";
            stream.write_all(message).await.unwrap(); // Send to server

            let mut buffer = [0; 64];
            let n = stream.read(&mut buffer).await.unwrap();
            stream.shutdown().await.expect("Can not close client tcp connection");
            assert_eq!(b"Hello, client!", &buffer[..n]);
        }
    }

    #[tokio::test]
    async fn server_udp_test() {
        let mut server2 = Server::new(DEFAULT_SERVER_IPENDPOINT,
            DEFAULT_SERVER_IPENDPOINT, 1);

        // UDP Server
        server2.start_udp_server_full(|socket| async move {
            let mut buf = vec![0u8; 512];
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((n, addr)) => {
                        assert_eq!(b"Hello, server!\n", &buf[..n], "OK Server.");

                        if let Err(_) = socket.send_to(b"Hello, Client!", &addr).await {
                            assert!(false, "❌ Failed to send response");
                        }
                    }
                    Err(_) => {
                        assert!(false, "❌ Error receiving data");
                        break;
                    }
                }
            }
            Ok(())
        })
        .await;

        // Give server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // UDP Client
        let client = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.send_to(b"Hello, server!\n", DEFAULT_SERVER_IPENDPOINT).await.unwrap();
        // 接收回显
        let mut buf = vec![0u8; 64];
        let n = client.recv(&mut buf).await.unwrap();
        assert_eq!(b"Hello, Client!", &buf[..n]);
    }
}