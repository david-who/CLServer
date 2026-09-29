use std::io::Result;
use std::net::SocketAddr;
use tokio::net::{ToSocketAddrs, UdpSocket};

// 封装UDP套接字
pub struct AsyncUDPClient {
    socket: UdpSocket,
}

impl AsyncUDPClient {
    // 创建一个新的UDP套接字
    pub async fn new<T: ToSocketAddrs>(local: T) -> Result<Self> {
        let socket = UdpSocket::bind(local).await?;
        Ok(Self { socket })
    }

    // 发送数据
    pub async fn send_to<T: ToSocketAddrs>(&self, buf: &[u8], dest: T) -> Result<usize> {
        self.socket.send_to(buf, dest).await
    }

    // 接收数据
    pub async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.socket.recv_from(buf).await
    }

    pub async fn connect<T: ToSocketAddrs>(&self, remote: T) -> Result<()> {
        self.socket.connect(remote).await
    }

    pub async fn send(&self, buf: &[u8]) -> Result<usize> {
        self.socket.send(buf).await
    }
}
