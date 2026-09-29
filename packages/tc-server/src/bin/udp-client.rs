use std::net::UdpSocket;
use std::io::{self, Write};
use std::env;

fn main() -> std::io::Result<()> {
    // Get server IP and port from environment variables or use defaults
    let server_ip = env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port = env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let server_addr = format!("{}:{}", server_ip, server_port);

    // Create a UDP socket
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    
    // Connect to the server
    socket.connect(&server_addr)?;
    println!("Connected to server at {}", server_addr);

    // Buffer for receiving data
    let mut buf = [0; 1024];

    loop {
        // Get input from user
        print!("Enter message (or 'quit' to exit): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        // Trim whitespace and check for quit command
        let input = input.trim();
        if input == "quit" {
            break;
        }

        // Send the message to the server
        socket.send(input.as_bytes())?;
        println!("Sent: {}", input);

        // Receive the echo from the server
        let amt = socket.recv(&mut buf)?;
        let response = std::str::from_utf8(&buf[..amt])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        println!("Received: {}", response);
    }

    Ok(())
} 