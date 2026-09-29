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
use tokio::net::TcpStream;
use std::sync::Arc; // Atomic Operations for thread-safe shared state
use std::sync::atomic::{AtomicBool, Ordering}; // Atomic Types
use tokio::io::{AsyncWriteExt, AsyncReadExt}; // TcpStream::shutdown() requires this trait


// 0.0.0.0::8080 所有网卡均被监听 (IPAny)
const DEFAULT_SERVER_IPENDPOINT: &'static str = "127.0.0.1:8080";

async fn handle_tcp_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0; 512];
    loop {
        match stream.read(&mut buffer).await {
            Ok(n) => {
                if n == 0 { // client closed connection
                    break;
                }

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


///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////


#[tokio::main]
async fn main() -> std::io::Result<()> {
/* CTRL+C: ARC */
    let running = Arc::new(AtomicBool::new(true));
    let shutdown_flag = Arc::clone(&running);

    ctrlc::set_handler(move || {
        shutdown_flag.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

/* LSB: arm64 == x86_64
    let x = 0x12345678i32;
    let bytes = x.to_le_bytes();
    print!("bytes: [");
    for u in bytes {
        print!("0x{:x}, ", u);
    }
    println!("]"); // LSB: [0x78, 0x56, 0x34, 0x12], MSB: [0x12, 0x34, 0x56, 0x78]

    // Convert back to i32
    let y: i32 = i32::from_ne_bytes(bytes);
    assert_eq!(x, y);
    println!("0x{:x}", y); // 0x12345678 ==> LSB
*/
    ///////////////////////////////////////////////////////////////////////////////

    let mut server = Server::new(DEFAULT_SERVER_IPENDPOINT,
            DEFAULT_SERVER_IPENDPOINT, 1);

        // TCP Server
    let handle = tokio::spawn(async move {
            server.start_tcp_server_full(handle_tcp_client).await;
        });

    ///////////////////////////////////////////////////////////////////////////////
/* CTRL+C: */
    while running.load(Ordering::SeqCst) {
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

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