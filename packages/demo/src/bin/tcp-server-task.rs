use std::future::Future;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Do not modify the address constant
const DEFAULT_SERVER_IPENDPOINT: &'static str = "127.0.0.1:8080";
// const TIMEOUT: Duration = Duration::from_secs(1);
const DEFAULT_TOTAL_CLIENTS: usize = 50;
const DEFAULT_TOTAL_MESSAGES_PER_CLIENT: usize = 50;

async fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0; 512];
    loop {
        match stream.read(&mut buffer).await {
            Ok(n) => {
                if n == 0 {
                    break;
                }
                println!("Received: {}", String::from_utf8_lossy(&buffer[..n]));

                let response = b"Hello, client!";
                stream.write_all(response).await.expect("Failed to write");
                stream.flush().await.expect("Failed to flush");
            }
            Err(e) => {
                println!("Error reading: {}", e);
                break;
            }
        }
    }
    Ok(())
}


#[cfg(false)] // ==> #if false
/* Fn 没有定义 Future trait, 所以无法直接使用 async fn 作为参数传入
 * 只能用同步的函数。如果实现 Future trait, 就需要用到范型, 又回到下面 start_server
 */
async fn invoke_server(handle: impl Fn(TcpStream) -> std::io::Result<()> + Clone + Send + 'static) {
    let (ip, _, _) = get_args();
    let listener = TcpListener::bind(ip).await.expect("Failed to bind");

    println!("Server listening on {}", listener.local_addr().unwrap());

    while let Ok((stream, _)) = listener.accept().await {
        let hand_ref = handle.clone();
        tokio::spawn(async move {
            if let Err(e) = hand_ref(stream) {
                println!("Error handling client: {}", e);
            }
        });
    }
}


async fn start_server<F, Fut>(proc: F)
where
    F: Fn(TcpStream) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = std::io::Result<()>> + Send + 'static,
{
    let (ip, _, _) = get_args();
    let listener = TcpListener::bind(ip).await.expect("Failed to bind");

    println!("Server listening on {}", listener.local_addr().unwrap());

    while let Ok((stream, _)) = listener.accept().await {
        println!("Connection established.");
        let handle = proc.clone();
        tokio::spawn(async move {
            if let Err(e) = handle(stream).await {
                println!("Error handling client: {}", e);
            }
        });
    }
}



#[tokio::main]
async fn main() -> std::io::Result<()> {
/*
    let (ip, _, _) = get_args();
//  let full_server_address = format!("{}:{}", DEFAULT_SERVER_ADDRESS, port);

    let listener = TcpListener::bind(ip).await.expect("Failed to bind");

    println!("Server listening on {}", listener.local_addr().unwrap());

    while let Ok((mut stream, _)) = listener.accept().await {
        // case 1: spawn a new closure to handle the client connection
        tokio::spawn(async move {
            let mut buffer = [0; 512];
            while let Ok(n) = stream.read(&mut buffer).await {
                if n == 0 {
                    break;
                }
                println!("Received: {}", String::from_utf8_lossy(&buffer[..n]));

                let response = b"Hello, client!";
                stream.write_all(response).await.expect("Failed to write");
                stream.flush().await.expect("Failed to flush");
            }
        });
        // case 2: spawn a new task to handle the client connection
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream).await {
                println!("Error handling client: {}", e);
            }
        });
    }
*/  start_server(handle_client).await;
    Ok(())
}

///<summary>Get the command line arguments for the server.</summary>
/// <returns> (port number, total clients, total messages)</returns>
#[allow(dead_code)]
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