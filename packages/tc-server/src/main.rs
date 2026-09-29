// 使用本地库 libecls.rlib
use ecls::server::{Server, Async};

use std::{net::SocketAddr, net::IpAddr, time::UNIX_EPOCH};
use tokio::time::Duration;
use clap::{Parser, ValueEnum};


///
/// UDP 软件协议: JD-61101
///   本地 IP: 192.168.118.118:15000    [CLS Server]
///   远程 IP: 192.168.118.005:16000    [CLS Host/Target]
/// CLS-TO-HOST: 上电初始化切换到力模式
///   DATA0: 0xA5, 帧头, 固定内容
///   DATA1: 0xA5, 帧头, 固定内容
///   DATA2: 0xA5, 帧头, 固定内容
///   DATA3: 0x01, 设备编号 
///   DATA4: 0x0E, 数据长度 (14): DATA5~DATA18
///★  DATA5: 0xXX, 0: 正常, 其他为故障码（详见乙方故障码对照表）
///   DATA6: 0x00, 预留数据位
///★  DATA7~DATA10, 脚蹬位移变化值: INT32[-18000,18000], 高位在前, 低位在后; 左脚向前负，右脚向前正 
///   DATA11~DATA18: 0x00, 预留数据位
///   DATA19: 和校验, 校验和 SUM[data3:data18]
/// 
/// 故障码对照表:
///   BIT0: 驱动器故障; BIT1: 触发急停; BIT2: 位置超限; BIT3: 力超限
///   BIT4: 速度超限
/// 
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BT_CLS_TO_HOST_T {
    data: [u8;20],
}

impl Default for BT_CLS_TO_HOST_T {
    fn default() -> Self {
        let mut d = Self {
            data: [0;20]
        };
        d.data[0] = 0xA5;
        d.data[1] = 0xA5;
        d.data[2] = 0xA5;
        d.data[3] = 0x01;
        d.data[4] = 0x0E;

        d
    }
}

impl BT_CLS_TO_HOST_T {
    pub fn new() -> Self {
        BT_CLS_TO_HOST_T::default()
    }

    pub fn get_pos(&self) -> i32 {
        let mut bytes = [0u8; 4];
        // Copy buffer into the bytes
        bytes.copy_from_slice(&self.data[7..11]);
        // Convert bytes (native endian) to i32: LSB
        i32::from_ne_bytes(bytes)
    }
    pub fn pos(&mut self, v: i32) -> &mut Self{
        // Convert i32 to bytes (native endian): LSB
        let bytes = v.to_ne_bytes();
        // Copy bytes into the buffer
        self.data[7..11].copy_from_slice(&bytes);
        self
    }

    pub fn get_sum(&self) -> i32 {
        (self.data[19] as u32) as i32
    }
    pub fn sum(&mut self, v: i32) -> &mut Self{
        self.data[19] = v as u8;
        self
    }
}

///
/// UDP 软件协议: JD-61101
///   本地 IP: 192.168.118.118:15000    [CLS Server]
///   远程 IP: 192.168.118.005:16000    [CLS Host]
/// CLS-FROM-HOST:
///   DATA0: 0xA5, 帧头, 固定内容
///   DATA1: 0xA5, 帧头, 固定内容
///   DATA2: 0xA5, 帧头, 固定内容
///   DATA3: 0x01, 设备编号 
///   DATA4: 0x0E, 数据长度 (14): DATA5~DATA18
///★  DATA5: 0x00: 正常, 0xAA: 清除故障
///   DATA6: 0x00, 预留数据位
///★  DATA7: 0x00: 正常, 0xAA: CLS 复位, 并回到中立位置(零位)
///   DATA8~DATA18: 0x00, 预留数据位
///   DATA19: 和校验, 校验和 SUM[data3:data18]
/// 
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BT_CLS_FROM_HOST_T {
    data: [u8;20],
}

impl Default for BT_CLS_FROM_HOST_T {
    fn default() -> Self {
        let mut d = Self {
            data: [0;20]
        };
        d.data[0] = 0xA5;
        d.data[1] = 0xA5;
        d.data[2] = 0xA5;
        d.data[3] = 0x01;
        d.data[4] = 0x0E;

        d
    }
}

impl BT_CLS_FROM_HOST_T {
    pub fn new() -> Self {
        BT_CLS_FROM_HOST_T::default()
    }

    pub fn get_fault(&self) -> i32 {
        (self.data[5] as u32) as i32
    }
    pub fn fault(&mut self, v: i32) -> &mut Self {
        self.data[5] = v as u8;
        self
    }

    pub fn get_reset(&self) -> i32 {
        (self.data[7] as u32) as i32
    }

    pub fn reset(&mut self, v: i32) -> &mut Self {
        self.data[7] = v as u8;
        self
    }

    pub fn get_sum(&self) -> i32 {
        (self.data[19] as u32) as i32
    }
    pub fn sum(&mut self, v: i32) -> &mut Self {
        self.data[19] = v as u8;
        self
    }
}


///
/// UDP 软件协议: JD-61101
///   调试 IP: 192.168.118.050:5050     [CLS Debugger]
///   远程 IP: 192.168.118.005:8080     [CLS Server]
/// HMI-TO-HOST/HMI-FROM-HOST: P_TcLCS_T
///
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BT_HMI_DATA_HOST_T {
    data: [u8;1400],
}


// 0.0.0.0::8080 所有网卡均被监听 (IPAny)
const DEFAULT_SERVER_IPENDPOINT: &'static str = "127.0.0.1:8080";
const DEFAULT_TARGET_IPENDPOINT: &'static str = "127.0.0.1:5050";


#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CMD {
    /// run the server
    Run,
    /// stop the server and exit application
    Stop,
    /// restart the server
    Restart,
    /// reset the cls parameters
    Reset,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(value_enum)]
    cmd: CMD,

    /// Print detail out infomation
    #[arg(short, long)]
    verbose: bool,

    /// Server IP Address
    #[arg(short = 'a', long, default_value = "192.168.118.118")]
    ip_address: IpAddr,

    /// Host IP Address
    #[arg(long, default_value = "192.168.118.5")]
    ip_host: IpAddr,

    /// Server port
    #[arg(short, long, default_value_t = 15000)]
    port: u16,

    /// Host/Remote port
    #[arg(long, default_value_t = 16000)]
    port_host: u16,

    /// Local debug port
    #[arg(long, default_value_t = 5050)]
    port_debug: u16,

    /// Local debug port
    #[arg(long, default_value_t = 8080)]
    port_host_debug: u16,
}



static mut NOW: u64 = 0;
static mut G_BUFFER : [u8; 128] = [0u8; 128];
static mut G_VERBOSE: bool = false;
fn is_verbose() -> bool {
    unsafe { G_VERBOSE }
}

fn recv(buf: &mut[u8], addr: SocketAddr) -> std::io::Result<()> {
    let msg = String::from_utf8_lossy(&buf[..]);
    if is_verbose() {
        println!("📩 Received from {}: {}", addr, msg);
    }

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
        G_BUFFER[..payload_len].copy_from_slice(payload);
    }

    unsafe { Some(&G_BUFFER[..payload_len]) }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let _args = Args::parse();

    unsafe {
        G_VERBOSE = _args.verbose;
        NOW = std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    }
    ///////////////////////////////////////////////////////////////////////////////

    let mut server = Server::new(DEFAULT_SERVER_IPENDPOINT,
        DEFAULT_TARGET_IPENDPOINT, 1);
    
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
    
    /// Struct Test
    #[test]
    fn struct_test() {
        let &mut to_host = BT_CLS_TO_HOST_T::new().sum(0x10E);
        let &mut fr_host = BT_CLS_FROM_HOST_T::new().fault(0x1AA);

        assert_eq!(to_host.get_sum(), 0x0E);
        assert_eq!(fr_host.get_fault(), 0xAA);
    }

}
