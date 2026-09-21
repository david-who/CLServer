//use std::sync::Arc;
use std::future::Future;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::io::AsyncWriteExt; // TcpStream::shutdown() requires this trait
use tokio::net::{TcpListener, TcpStream, UdpSocket};

use tokio::time::{interval, Duration};

/*
///<summary>
/// Trait for handling TCP connections asynchronously.
/// custom must implemente this trait to process incoming TCP streams.
/// </summary>
pub trait ConnectionHandler: Send + Sync + 'static {
    async fn handle(&self, stream: TcpStream) -> io::Result<()>;
}

pub trait ServerAsync {
    async fn start_tcp_server<H>(
        &self,
        proc: H,
    );
}
*/

#[allow(async_fn_in_trait)]
pub trait Async {
    async fn start_tcp_server_full<F, Fut>( &mut self, proc: F )
    where
    F: Fn(TcpStream) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = std::io::Result<()>> + Send + 'static;

    async fn start_udp_server_full<F, Fut>( &mut self, proc: F )
    where
    F: Fn(UdpSocket) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = std::io::Result<()>> + Send + 'static;

    async fn start_udp_server<'a, F, D>( &mut self, t: Duration, proc: F, prod: D )
    where
    F: Fn(&mut[u8], SocketAddr) -> std::io::Result<()>,
    D: Fn() -> Option<&'a[u8]>;
}

pub struct Server
{
    counter: usize,         // 当前连接数
    nclient: usize,         // 最大连接数
    addr_lo: SocketAddr,    // 监听地址
    #[allow(unused)]        // 仅用于 UDP
    addr_rm: SocketAddr,    // 目标地址
}

impl Server {
    pub fn new<T: ToSocketAddrs>(local: T, target: T, total: usize) -> Self {
        let num = if total == 0 { 1 } else { total };
        let local = if let Ok(mut addr) = local.to_socket_addrs() {
            addr.nth(0).unwrap()
        } else {
            SocketAddr::from(([0, 0, 0, 0], 8080)) // default to anyhost:8080
        };
        let target = target.to_socket_addrs().unwrap().nth(0).expect("Failed to resolve target address");
        Self { counter: 0, nclient: num, addr_lo: local, addr_rm: target }
    }
}



impl Async for Server {
    async fn start_tcp_server_full<F, Fut>( &mut self, proc: F )
    where
        F: Fn(TcpStream) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = std::io::Result<()>> + Send + 'static
    {
        let listener = TcpListener::bind(self.addr_lo).await.expect("Failed to bind");

        println!("TCP listening on {}", listener.local_addr().unwrap());

        while let Ok((mut stream, _)) = listener.accept().await {
            if self.counter > self.nclient {
                #[cfg(debug_assertions)]
                {
                println!("Maximum number of clients reached. Rejecting connection.");
                stream.shutdown().await.expect("Failed to shutdown stream");
                continue;
                }
            }
            self.counter += 1;
            #[cfg(debug_assertions)]
            println!("Connection established: {}", self.counter);

            let handle = proc.clone(); // 必须实现?!
            tokio::spawn(async move {
                handle(stream).await.expect("Error handling client");
            });
        }
    }

    async fn start_udp_server_full<F, Fut>( &mut self, proc: F )
    where
    F: Fn(UdpSocket) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = std::io::Result<()>> + Send + 'static
    {
        let socket = tokio::net::UdpSocket::bind(self.addr_lo).await.expect("Failed to bind UDP socket");
        println!("UDP listening on {}", socket.local_addr().unwrap());

        let handle = proc.clone();
        tokio::spawn(async move {
            handle(socket).await.expect("Error handling UDP packet");
        });
    }

    async fn start_udp_server<'a, F, D>( &mut self, t: Duration, proc: F, prod: D )
    where
    F: Fn(&mut[u8], SocketAddr) -> std::io::Result<()>,
    D: Fn() -> Option<&'a[u8]>
    {
        let socket = tokio::net::UdpSocket::bind(self.addr_lo).await
            .expect(format!("Failed to bind UDP: {}", self.addr_lo).as_str());
        println!("UDP listening on {}", socket.local_addr().unwrap());

        // Buffer for incoming data
        let mut buf = [0u8; 512];
        let mut interval = interval(t);

        loop {
            tokio::select! {
                _ = async { // 异步接收
                    match socket.recv_from(&mut buf).await {
                        Ok((len, addr)) => {
                            self.counter += 1;
                            proc(&mut buf[..len], addr).expect("Error processing received data");
                        }
                        Err(e) => {
                            eprintln!("❌ Error receiving data: {}", e);
                        }
                    }
                } => {}
                _ = interval.tick() => { // 定时发送
                    if let Some(bytes) = prod() {
                        if let Err(e) = socket.send_to(bytes, self.addr_rm).await {
                            eprintln!("❌ Error sending data: {}", e);
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => { // CTRL + C
                    println!("\n\tShutting down server...");
                    break;
                }
            }
        }
    }
}
