/// tc-server: main.rs
/// 使用本地库 libecls.rlib
use ecls::server::{Server, Async};

use std::{
    net::{IpAddr, SocketAddr},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        OnceLock
    },
    time::UNIX_EPOCH,
};
use tokio::{
    net::UdpSocket,
    io::AsyncWriteExt,
    time::Duration
};
// Command line argument parser
use clap::{Parser, ValueEnum};
// Share memory for Params and IOs
use ecls::{
    ecpm::EcPmShm,
    ecshm::{EcIoShm, BT_CLSIn_T},
};


/// Constants for server operation
const T_CYC_MS: u64 = 1000;  // million seconds
const T_TIMEOUT_S: u64 = 10; // seconds
const STOP_PACKET: &'static str = "ECLS:STOP💖"; // To Stop the server!
const K_RX_SIZE: usize = size_of::<BT_CLS_FROM_HOST_T>();

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

impl BT_CLS_TO_HOST_T {
    pub const fn zero() -> Self {
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
    
    pub fn new() -> Self {
        BT_CLS_TO_HOST_T::zero()
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

impl BT_CLS_FROM_HOST_T {
    pub const fn zero() -> Self {
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

    pub fn new() -> Self {
        BT_CLS_FROM_HOST_T::zero()
    }

    pub fn get_header(&self) -> u32 {
        let bytes = [self.data[0], self.data[1], self.data[2], self.data[4]];
        let u = u32::from_ne_bytes(bytes.try_into().unwrap());
        u
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

    /// Print detail out infomation
    #[arg(long)]
    hmi_server: bool,

    /// Server IP Address
    #[arg(short = 'a', long, default_value = "192.168.118.118")]
    ip_local: IpAddr,

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


// Can not directly use static mut.
// #[allow(static_mut_refs)] suppressing the warning/error.
// OnceLock to share static mut reference.
static G_ECSHM: OnceLock<EcIoShm> = OnceLock::new();
static G_ECPM : OnceLock<EcPmShm> = OnceLock::new();

// NO need to use OnceLock<Mutex<T>> for sharing global objects,
// using static mut is enough, because only one thread will access
// the global data at a time. otherwise, should use Mutex to protect the data.
//static G_TO_HOST: OnceLock<Mutex<BT_CLS_TO_HOST_T>>   = OnceLock::new();
//static G_FR_HOST: OnceLock<Mutex<BT_CLS_FROM_HOST_T>> = OnceLock::new();
static mut GB_TO_HOST: BT_CLS_TO_HOST_T   = BT_CLS_TO_HOST_T::zero();
static mut GB_FR_HOST: BT_CLS_FROM_HOST_T = BT_CLS_FROM_HOST_T::zero();
static mut G_CTRL_IN : BT_CLSIn_T = BT_CLSIn_T {
    cmd     : 0,
    fwd_fric: 0.0,
    pcmd    : 0.0,  
    trav_a  : 15.0,
    trav_b  :-15.0,
    mass    : 0.01,  
    bf      : 0.0,   
    fh      : 0.0,   
    vap     : 0.0,   
    sw_fn   : 0,
    v_trim  : 0.0,
    fa_off  : 0.0,
    fa_grad : 0.0,
    p0_trim : 0.0,
    spare1  : 0.0,
    spare2  : 0.0,
    spare3  : 0.0,
};

static NOW: AtomicU64 = AtomicU64::new(0);
static G_VERBOSE: AtomicBool = AtomicBool::new(false);
static G_RUNNING: AtomicBool = AtomicBool::new(true);

fn is_running() -> bool {
    G_RUNNING.load(Ordering::Relaxed)
}

fn is_verbose() -> bool {
    G_VERBOSE.load(Ordering::Relaxed)
}

// EcIoShm: Recieve data from CLS Host, and write to share memory
fn recv(buf: &mut[u8], addr: SocketAddr) -> std::io::Result<()> {
    match buf.len() {
        K_RX_SIZE => unsafe { // BT_CLS_FROM_HOST_T
            // 1) Copy data to GB_FR_HOST
            #[allow(static_mut_refs)] // FIXME: Unsafe static mutable reference
            let host = &mut GB_FR_HOST;
            host.data.copy_from_slice(buf);

            // 2) verify header
            if host.get_header() != 0x0EA5A5A5 {
                if is_verbose() {
                    eprintln!("❌ Invalid header from {}: {:X?}", addr, buf);
                }
                return Ok(());
            }

            // 3) process host data
            G_CTRL_IN.cmd = 1;

            // 4) write to share memory
            let ecshm = G_ECSHM.get().expect("EcIoSHM not initialized");
            #[allow(static_mut_refs)] // FIXME
            let c = &G_CTRL_IN;
            ecshm.set_ctrl_in(c, 0).expect("can not set shm ctrl_in");
        },
        _ => {
            let msg = String::from_utf8_lossy(&buf[..]);
            if is_verbose() {
                eprintln!("📩 Received from {}: {}", addr, msg);
            }
        }
    }

    Ok(())
}

// EcIoShm: Read data from share memory, and send to CLS Host
fn send<'a>() -> Option<&'a [u8]> {
/* FOR Debugging:
    #[cfg(debug_assertions)]
    {
        let t0 = NOW.load(Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH + Duration::new(t0, 0))
            .expect("Time went backwards");
        let msg = format!("hello client: {:.2}", now.as_secs_f32());
        let payload = msg.into_bytes();
        let leaked = Box::leak(payload.into_boxed_slice());
        Some(leaked)
    }
    #[cfg(not(debug_assertions))]
*/
    unsafe {
        Some(&GB_TO_HOST.data[..])
    }
}

// EcShm: Handle HMI client connection, receive data and send response
async fn handle_hmi_client(socket: UdpSocket) -> std::io::Result<()> {
    let mut buf = vec![0u8; 512];
    while is_running() {
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
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Step 1: Parse command line arguments
    let args = Args::parse();
    // 1.1 Verbose output
    G_VERBOSE.store(args.verbose, Ordering::Relaxed);
    NOW.store(
        std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        Ordering::Relaxed,
    );
    // 1.2 Run parsed Command
    match args.cmd {
        CMD::Run => {},
        CMD::Stop => {
            match tokio::time::timeout(tokio::time::Duration::from_secs(T_TIMEOUT_S),
                    tokio::net::TcpStream::connect(SocketAddr::new(args.ip_local, args.port))).await
            {
                Ok(st) => {
                    let mut stream = st.unwrap();
                    stream.write_all(STOP_PACKET.as_bytes())
                        .await.expect("Failed to send stop packet");
                }
                Err(_) => {} // timeout
            }
            return Ok(())
        },
        CMD::Restart => println!("🔄 Restarting server..."),
        CMD::Reset => {
            println!("🔧 Resetting CLS parameters...");
        }
    }
    ///////////////////////////////////////////////////////////////////////////////

    // Step 2: Init Global Data, Configure...

    // Step 3: Open TcPmSHM share-memory
    let ecpm = G_ECPM.get_or_init(|| { // Initialize Once
        if let Ok(ec) = EcPmShm::open(true) {
            ec
        } else {
            EcPmShm::create(0).expect("Failed to create share memory file")
        }
    });
    { // check if the share file is valid
        let res = ecpm.map_as_mut();
        assert!(res.is_ok(), "Can not get EC PM_Buffer: {:?}", res);
        let mbuf = res.unwrap();
        let tag = mbuf.get_tag();
        assert_eq!(tag.as_bytes(), b"TcPM_Buffer 1.0\0");
    }

    // Step 4: Open EcIoSHM share-memory
    let ecshm = G_ECSHM.get_or_init(|| { // Initialize Once
        if let Ok(ec) = EcIoShm::open(true) {
            ec
        } else {
            EcIoShm::create(0).expect("Failed to create share memory")
        }
    });
    { // check if the share memory is valid
        let buf = ecshm.bytes_as_readonly().expect("can not get cls_in");
        let bytes = buf[0..4].try_into().unwrap();
        let sz = u32::from_ne_bytes(bytes) as usize;
        assert_eq!(sz, std::mem::size_of::<ecls::ecshm::EcIO_Buffer_T>(), "EcIoSHM size mismatch");
    }
    unsafe {// set initial values for BT_CLSIn_T
        #[allow(static_mut_refs)] // FIXME: Unsafe static mutable reference
        let c = &G_CTRL_IN;
        ecshm.set_ctrl_in(c, 0).expect("can not set BT_CLSIn_T to EcIoSHM");
    }

    // Step 5: Start the UDP server for IO relay
    let mut server = Server::new(
        SocketAddr::new(args.ip_local, args.port),
        SocketAddr::new(args.ip_host, args.port_host),
        1 );
    
    let handle = tokio::spawn(async move {
        server.start_udp_server(Duration::from_millis(T_CYC_MS), recv, send).await;
    });

    // Step 6: Start the server for HMI
    if args.hmi_server {
        let mut hmi_server = Server::new(
            SocketAddr::new(args.ip_local, args.port_debug),
            SocketAddr::new(args.ip_host, args.port_host_debug),
            1 );
        
        tokio::spawn(async move {
            hmi_server.start_udp_server_full(handle_hmi_client).await;
        });
    }

    ///////////////////////////////////////////////////////////////////////////////

    // Wait for the server task to end
    match handle.await {
        Ok(_) => println!("🛑 Shut down."),
        Err(e) => eprintln!("Task failed: {}", e),
    }
    // Stop the hmi server
    G_RUNNING.store(false, Ordering::Relaxed);

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    // 0.0.0.0::8080 所有网卡均被监听 (IPAny)
    const DEFAULT_SERVER_IPENDPOINT: &'static str = "127.0.0.1:8080";
//  const DEFAULT_TARGET_IPENDPOINT: &'static str = "127.0.0.1:5050";

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
        let mut to_host = BT_CLS_TO_HOST_T::new();
        to_host.sum(0x10E);

        let mut fr_host = BT_CLS_FROM_HOST_T::new();
        fr_host.fault(0x1AA);

        assert_eq!(to_host.get_sum(), 0x0E);
        assert_eq!(fr_host.get_fault(), 0xAA);
    }

}
