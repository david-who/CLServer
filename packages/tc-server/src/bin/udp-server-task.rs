use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::signal;
use std::error::Error;


// Do not modify the address constant
const DEFAULT_SERVER_IPENDPOINT: &'static str = "127.0.0.1:8080";
// const TIMEOUT: Duration = Duration::from_secs(1);
const DEFAULT_TOTAL_CLIENTS: usize = 50;
const DEFAULT_TOTAL_MESSAGES_PER_CLIENT: usize = 50;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Get IP and port from environment variables or use defaults
    let (addr, _, _) = get_args();
    // Create a UDP socket
    let socket = UdpSocket::bind(&addr).await?;
    println!("✅ UDP server listening on {}", socket.local_addr()?);

    // Buffer for incoming data
    let mut buf = vec![0u8; 512];

    // Graceful shutdown signal
    tokio::select! { // 模式匹配
        _ = async {
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        let msg = String::from_utf8_lossy(&buf[..len]);
                        println!("📩 Received from {}: {}", addr, msg);

                        // Echo back to sender
                        if let Err(e) = socket.send_to(msg.as_bytes(), &addr).await {
                            eprintln!("❌ Failed to send response: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Error receiving data: {}", e);
                    }
                }
            }
        } => {}
        _ = signal::ctrl_c() => {
            println!("🛑 Shutdown signal received.");
        }
    }

    Ok(())
}


///<summary>Get the command line arguments for the server.</summary>
/// <returns> (sockAddr, total clients, total messages)</returns>
fn get_args() -> (SocketAddr, usize, usize) {
    let mut args = std::env::args().skip(1);

    (
        args.next()
            .map(|a| a.parse().expect("invalid SocketAddr"))
            .unwrap_or_else(|| DEFAULT_SERVER_IPENDPOINT.parse().unwrap() ),
        args.next()
            .map(|a| a.parse().expect("invalid usize for total clients"))
            .unwrap_or_else(|| DEFAULT_TOTAL_CLIENTS),
        args.next()
            .map(|a| {
                a.parse()
                    .expect("invalid usize for total messages per client")
            })
            .unwrap_or_else(|| DEFAULT_TOTAL_MESSAGES_PER_CLIENT),
    )
}