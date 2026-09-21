use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::str;
//use std::env;

// Do not modify the address constant
const DEFAULT_SERVER_IPENDPOINT: &'static str = "127.0.0.1:8080";
// const TIMEOUT: Duration = Duration::from_secs(1);
const DEFAULT_TOTAL_CLIENTS: usize = 50;
const DEFAULT_TOTAL_MESSAGES_PER_CLIENT: usize = 50;

fn main() -> std::io::Result<()> {
    // atomic flag for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let shutdown_flag = Arc::clone(&running);

    ctrlc::set_handler(move || {
        shutdown_flag.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");
    
    // Get IP and port from environment variables or use defaults
/*  let ip   = env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", ip, port);
*/  let (addr, _, _) = get_args();

    // Create a UDP socket
    let socket = UdpSocket::bind(&addr)?;
    println!("UDP Server listening on {}", addr);

    // Buffer for receiving data
    let mut buf = [0; 512];

    loop {
        // Receive data from client
        let (amt, src) = socket.recv_from(&mut buf)?;
        
        // Convert received bytes to string
        let received = str::from_utf8(&buf[..amt])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        println!("Received from {}: {}", src, received);

        // Echo the message back to the client
        socket.send_to(&buf[..amt], &src)?;
        println!("Echoed back to {}", src);

        // exit the loop if the running flag is set to false (for graceful shutdown)
        if !running.load(Ordering::SeqCst) {
            println!("Shutting down server...");
            break;
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