//#[path ="./error.rs"] // can find it in current dir.
mod error;
use error::ShmemError; // custom error => Err(ShmemError)

use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, AtomicU8};
use libc::{c_void, c_char, c_uint, off_t};
use libc::{O_CREAT, O_RDWR, O_RDONLY, S_IRUSR, S_IWUSR, PROT_READ, PROT_WRITE, MAP_FAILED, MAP_SHARED};


/// CLS Model Control Inputs <== Host
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BT_CLSIn_T {
    /**
     * Low 8-bits Mode Command:
     *  0: OFF
     *  1: Force Loop X1 / Normal Loading Control
     *  2: FreeTest
     *  3: Spring Test
     *  4: 5-Section Spring Test
     *  5: Force Loop X2 / NONE
     *  6: Position Mode
     *  7: Clock Position
     *  8: Jagging Move / Manually Move
     *  9: Veloctity Mode
     *  10: Reset
     */
    pub cmd: u32,

    /// Forward Friction/Jam Override Level [N]:
    /// 1. used as control friction in NORMAL mode.
    /// 2. used as override level in JAM mode.
    pub fwd_fric: f32,
    pub pcmd: f32,  // FWD. Jam Position [deg]: Position command from host for backdriving in JAM/REPLAY mode.
    pub trav_a: f32,// Pos Travel Above Limit: Very stiff stop at positive direction.
    pub trav_b: f32,// Pos Travel Below Limit: Very stiff stop at negative direction.
    pub mass: f32,  // FWD. Added Mass [N/(deg/s^2)]: Additional mass to the minimal or tuned value.

    pub bf : f32,   // FWD. Added Damping [N/(deg/s)]: Damping term added to minimal damping set in ACU.
    pub fh : f32,   // Force Input [N]: General purpose force control, normally for backdriving.
    pub vap: f32,   // Autopilot Velocity [deg/s].

    /**
     * Autopilot Function and More:
     *  0  OFF    OFF
     *  1  OFF    ON
     *  2  ON     OFF
     *  3  ON     ON
     *  4  RESET  OFF
     *  BIT16: Release Force
     */
    pub sw_fn  : u32,
    pub v_trim : f32,   // Trim Velocity [deg/s].
    pub fa_off : f32,   // Aero Force Offset [N].
    pub fa_grad: f32,   // Aero Force and Additive Force [N].
    pub p0_trim: f32,   // Trim Init Position [deg].

    pub spare1 : f32,   // Reserved 1
    pub spare2 : f32,   // Reserved 2
    pub spare3 : f32,   // Reserved 3
}

/// CLS Model Control Output ==> Host
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BT_CLSOut_T {
    /**
     * Channel State Word:
     *  E_CHN_OFF          :=   0,  // 未使能状态
     *  E_CHN_ENABLE       :=   1,  // 使能状态
     *  E_CHN_FORCELOOP1   :=   2,  // 力回路模式状态 X1
     *  E_CHN_FREETEST     :=   3,  // 力模式自由状态
     *  E_CHN_SPRINGTEST   :=   4,  // 力模式弹簧状态
     *  E_CHN_SEC5SPRING   :=   5,  // 五段弹簧测试模式
     *  E_CHN_FORCELOOP2   :=   6,  // 力回路模式状态 X2
     *  E_CHN_POSLOOP      :=   7,  // 位置回路模式状态
     *  E_CHN_JAGMENT      :=   8,  // 位置模式 Jag 状态
     *  E_CHN_POSLOCK      :=   9,  // 位置模式 Lock 状态
     *  E_CHN_VELLOOP      :=  10,  // 速度回路模式状态
     *  E_CHN_HOMING       :=  16,  // 归零状态,   BIT5=1
     *  E_CHN_INIT         :=  32,  // 初始化状态, BIT4=1
     *  E_CHN_FAULT        :=  64,  // 故障状态,   BIT6=1
     *  E_CHN_RESET        := 128,  // 复位状态,   BIT7=1
     *  E_CHN_STOP         := 255   // 停止状态,   BIT7=1
     */
    pub status: i32,

    /**
     * Channel Safety Status:
     *  0: Center
     *  1: L0TravA (Up Limit),     3: L0TravA Hold
     *  2: L0TravB (Low Limit),    4: LBTrav2 Hold
     *  5: Center of P0Trim
     *  6: L0Trim,                 7: -L0Trim
     *  8: KFmax (Up Limit),     10: KFmax Hold
     *  9: -KFmax (Low Limit),   11: -KFmax Hold
     *  12: KFLMT (SensorF),     13: KFLMT Hold
     *  14: KVmax (FwdVel),      15: KVmax Hold
     *  16: F-Sensor Ready: 0 Ready, 1 Preparing.
     */
    pub safety: i32,

    /// Fading flag: 1 = fading active, 0 = fading not active.
    pub is_fading: i32,

    /// FWD. Position [deg].
    pub fwd_pos: f32,

    /// FWD. Velocity [deg/s].
    pub fwd_vel: f32,

    /// FWD. Force [N].
    pub fwd_force: f32,

    /// Cable Force [N], invert of FWD. Force.
    pub cable_force: f32,

    /// Trim Position [deg].
    pub trim_pos: f32,

    /// AFT. Position: Differs from control position because of cable stretch,
    /// cable deadband and aft travel limits.
    pub aft_pos: f32,

    /// Motor Position [deg].
    pub motor_pos: f32,

    /// Motor Velocity [deg/s].
    pub motor_vel: f32,

    /// Sensor Force [N].
    pub sensor_f: f32,

    /// Command Force/Torque.
    pub tcmd: f32,
}


/// CLS Model Sensor Inputs <== EtherCAT Slaves.
#[allow(non_snake_case)]
#[repr(C)] // 保证与 C 内存布局一致
#[derive(Debug, Clone, Copy)]
pub struct BT_DriveIn_T
{
    pub KqS: i8,  // Sensor Status (ex. Kistler Amplifier Status).
    pub SW : u16, // Driver Status Word.
    pub Tq : i16, // Torque Feedback
    pub Pos: i32, // Position Feedback
    pub Vel: i32, // Velocity Feedback
}

/// CLS Model Drive Output ==> EtherCAT Slaves
#[allow(non_snake_case)]
#[repr(C)] // 保证与 C 内存布局一致
#[derive(Debug, Clone, Copy)]
pub struct BT_DriveOut_T
{
    pub KqCtrl  : i8,   // Sensor Control (ex. Kistler Amplifier Control).
    pub OpMode  : i8,   // Drive  Operation Mode
    pub ControlW: u16,  // Drive  Control Word
    pub TargetQ : i16,  // Target Torque Command
    pub TargetV : i32,  // Target Velocity Command
    pub TargetP : i32,  // Target Position Command
    pub TcP     : u16,  // Drive  Positive Force out Limit: 60E0 (1000 x %)
    pub TcN     : u16,  // Drive  Negative Force out Limit: 60E1 (1000 x %)
}


/// CLS IO
#[repr(C)] // 保证与 C 内存布局一致
#[derive(Debug, Clone, Copy)]
pub struct U_TcLCS_T {
    pub ctr_in: BT_CLSIn_T,   // 控制器输入, from HOST
    pub drv_in: BT_DriveIn_T, // 驱动器输入, from Driver
}

/// External outputs (root outports fed by signals with default storage)
#[repr(C)] // 保证与 C 内存布局一致
#[derive(Debug, Clone, Copy)]
pub struct Y_TcLCS_T {
    pub ctr_out: BT_CLSOut_T,   // 控制器输出, to HOST
    pub drv_out: BT_DriveOut_T, // 驱动器输出, to Driver
}

impl Default for U_TcLCS_T {
    fn default() -> Self {
        Self {
            ctr_in: BT_CLSIn_T {
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
            },
            drv_in: BT_DriveIn_T {
                KqS: 0,
                SW : 0,
                Tq : 0,
                Pos: 0,
                Vel: 0,
            }
        }
    }
}

impl Default for Y_TcLCS_T {
    fn default() -> Self {
        Self {
            ctr_out: BT_CLSOut_T {
                status      : 0,
                safety      : 0,
                is_fading   : 0,
                fwd_pos     : 0.0,
                fwd_vel     : 0.0,
                fwd_force   : 0.0,
                cable_force : 0.0,
                trim_pos    : 0.0,
                aft_pos     : 0.0,
                motor_pos   : 0.0,
                motor_vel   : 0.0,
                sensor_f    : 0.0,
                tcmd        : 0.0,
            },
            drv_out: BT_DriveOut_T {
                KqCtrl  : 0,
                OpMode  : 0,
                ControlW: 6,
                TargetQ : 0,
                TargetV : 0,
                TargetP : 0,
                TcP     : 1000,
                TcN     : 1000,
            }
        }
    }
}


/**
 *  @brief TC CLS Io Buffer [176 bytes]
 *  @note use initializer list to ensure all members are initialized to zero.
 *    ex. EcIO_Buffer_T buf = {};
 *    -std=c++20 -O1 may optimize: "rep stosq"
 */
#[allow(non_snake_case)]
#[repr(C)] // 保证与 alignas(8) 一致
#[derive(Debug)]
pub struct EcIO_Buffer_T {
    dwSize: u32, // readonly, sizeof(EcIO_Buffer_T)
    nChans: u16, // readonly, channel number

    pub cls_in_updated: AtomicBool, // cls_in update flag
    pub cls_in_splock : AtomicU8,   // cls_in spin lock
    pub cls_in: [U_TcLCS_T; Self::K_SIZE], // 4x17bytes = 68 bytes

    pub cls_out_updated: AtomicBool,
    pub cls_out_splock : AtomicU8,
    pub cls_out: [Y_TcLCS_T; Self::K_SIZE], // 4x13 *k bytes = 52 bytes
}

impl EcIO_Buffer_T {
    const K_SIZE: usize = 1; // 通道数量: 1~10

    /// 创建一个默认初始化的缓冲区
    pub fn new() -> Self {
        let buf = EcIO_Buffer_T {
            dwSize: size_of::<EcIO_Buffer_T>() as u32,
            nChans: Self::K_SIZE as u16,
            cls_in_updated: AtomicBool::new(false),
            cls_in_splock : AtomicU8::new(0),
            cls_in: [U_TcLCS_T::default(); Self::K_SIZE],
            cls_out_updated: AtomicBool::new(false),
            cls_out_splock : AtomicU8::new(0),
            cls_out: [Y_TcLCS_T::default(); Self::K_SIZE],
        };

        buf
    }

    /// 获取通道数
    pub fn get_chans_num(&self) -> usize {
        self.nChans as usize
    }

    /// size_of
    pub fn size_of() -> usize {
        std::mem::size_of::<Self>()
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct EcIoShm {
    mct: u32,   // Lock timeout in counts.
    mfd: RawFd, // Shared memory file descriptor
    msz: usize, // Shared memory size in bytes
    mpbuf: *mut EcIO_Buffer_T,  // Shared memory address
}

impl EcIoShm {
    const LOCK_TIME_OUT: u32 = 10;
    #[cfg(target_os = "linux")]
    const SHM_PATH: &'static str = "EcIoSHM\0";  // It is a path under /dev/shm in Linux.
    #[cfg(not(target_os = "linux"))]
    const SHM_PATH: &'static str = "/ECI/EcIoSHM/2.0\0";

    /// Static Method: delete the share-memory
    pub fn delete() -> Result<(), std::io::Error> {
        unsafe {
            let m = libc::shm_unlink(Self::SHM_PATH.as_ptr() as *const c_char);
            if m != 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    /// 创建共享内存
    pub fn create(size: usize) -> Result<Self, std::io::Error> {
        let sz = if size >= size_of::<EcIO_Buffer_T>() {
                size
            } else {
                size_of::<EcIO_Buffer_T>()
            };
        unsafe {
            let fd = libc::shm_open(
                Self::SHM_PATH.as_ptr() as *const c_char,
                O_CREAT | O_RDWR,
                (S_IRUSR | S_IWUSR) as c_uint);
            if fd < 0 {
                return Err(std::io::Error::last_os_error());
            }

            if libc::ftruncate(fd, sz as off_t) != 0 {
                libc::close(fd);
                return Err(std::io::Error::last_os_error());
            }

            let addr = libc::mmap(
                std::ptr::null_mut(),
                sz,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd,
                0 as off_t,
            );

            if addr == MAP_FAILED {
                libc::close(fd);
                return Err(std::io::Error::last_os_error());
            }

            let buf = EcIO_Buffer_T::new();
            let ptr = &buf as *const EcIO_Buffer_T as *const c_void;
            addr.copy_from(ptr, EcIO_Buffer_T::size_of());

            Ok(EcIoShm{
                mct: Self::LOCK_TIME_OUT,
                mfd: fd,
                msz: sz,
                mpbuf: addr as *mut EcIO_Buffer_T,
            })
        }
    }

    /// 打开共享内存
    pub fn open(writable: bool) -> Result<Self, std::io::Error> {
        let mut mflag = PROT_READ;
        let mut oflag = O_RDONLY;
        if writable {
            mflag |=  PROT_WRITE;
            oflag  = O_RDWR;
        }
        unsafe {
            let fd = libc::shm_open(
                Self::SHM_PATH.as_ptr() as *const c_char, //TODO: 字符串必须以 NULL 结尾
                oflag);
            if fd < 0 {
                return Err(std::io::Error::last_os_error());
            }

            let mut sz = size_of::<EcIO_Buffer_T>();
            let addr = libc::mmap(
                std::ptr::null_mut(),
                sz,
                PROT_READ,
                MAP_SHARED,
                fd,
                0 as off_t,
            );

            if addr == MAP_FAILED {
                libc::close(fd);
                return Err(std::io::Error::last_os_error());
            } else {
                let ps = addr as *const u32;
                sz = *ps as usize;
                libc::munmap(addr, size_of::<EcIO_Buffer_T>());
            }
            if sz < size_of::<EcIO_Buffer_T>() {
                libc::close(fd);
                return Err(std::io::Error::new(std::io::ErrorKind::Other,
                    ShmemError::MapSizeError));
            }

            let addr = libc::mmap(
                std::ptr::null_mut(),
                sz,
                mflag,
                MAP_SHARED,
                fd,
                0 as off_t,
            );
            if addr == MAP_FAILED {
                libc::close(fd);
                Err(std::io::Error::last_os_error())
            } else {
                Ok(EcIoShm{
                    mct: Self::LOCK_TIME_OUT,
                    mfd: fd,
                    msz: sz,
                    mpbuf: addr as *mut EcIO_Buffer_T,
                })
            }
        }
    }

    /// 关闭共享内存
    pub fn close(&mut self) -> Result<(), std::io::Error> {
        let mut m = 0;
        unsafe {
            if !self.mpbuf.is_null() {
                m = libc::munmap(self.mpbuf as *mut c_void, self.msz);
                self.mpbuf = std::ptr::null_mut();
            }
            if self.mfd > 0 {
                libc::close(self.mfd);
                self.mfd = -1;
            }
        }
        
        if m == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// 获取共享数据只读副本
    pub fn map_as_readonly(&self) -> Result<&EcIO_Buffer_T, ShmemError> {
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        }
        if self.mpbuf.align_offset(std::mem::align_of::<EcIO_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { Ok(
            self.mpbuf.cast::<EcIO_Buffer_T>().as_ref().unwrap() )
        }
    }

    /// 获取共享数据
    #[cfg(test)]
    pub fn map_as_mut(&self) -> Result<&EcIO_Buffer_T, ShmemError> {
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        }
        if self.mpbuf.align_offset(std::mem::align_of::<EcIO_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { Ok(
            self.mpbuf.cast::<EcIO_Buffer_T>().as_mut().unwrap() )
        }
    }

    /// 获取共享数据只读区域
    pub fn bytes_as_readonly(&self) -> Result<&[u8], ShmemError> {
        if self.mpbuf.is_null() ||
           self.msz < size_of::<EcIO_Buffer_T>() {
            return Err(ShmemError::MapSizeZero);
        }

        unsafe { Ok(
            std::slice::from_raw_parts(self.mpbuf as *const u8, self.msz) )
        }
    }

    /// 获取共享数据读写区域
    #[cfg(test)]
    pub fn bytes_as_mut(&self) -> Result<&mut[u8], ShmemError> {
        if self.mpbuf.is_null() ||
           self.msz < size_of::<EcIO_Buffer_T>() {
            return Err(ShmemError::MapSizeZero);
        }

        unsafe { Ok(
            std::slice::from_raw_parts_mut(self.mpbuf as *mut u8, self.msz) )
        }
    }

    ///////////////////////////////////////////////////////////////////////////

    pub fn get_ctrl_in(&self, idx: usize) -> Result<&BT_CLSIn_T, ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcIO_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcIO_Buffer_T>().as_ref().unwrap();

            Ok( &pmb.cls_in[idx].ctr_in )
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_ctrl_in(&self, c: &BT_CLSIn_T, idx: usize) -> Result<(), ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcIO_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcIO_Buffer_T>().as_mut().unwrap();
            pmb.cls_in[idx].ctr_in.clone_from(c);
        }

        Ok(())
    }

    pub fn get_drive_in(&self, idx: usize) -> Result<&BT_DriveIn_T, ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcIO_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcIO_Buffer_T>().as_ref().unwrap();

            Ok( &pmb.cls_in[idx].drv_in )
        }
    }

    #[cfg(test)]
    pub fn set_drive_in(&self, d: &BT_DriveIn_T, idx: usize) -> Result<(), ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcIO_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcIO_Buffer_T>().as_mut().unwrap();
            pmb.cls_in[idx].drv_in.clone_from(d);
        }

        Ok(())
    }

    pub fn get_ctrl_out(&self, idx: usize) -> Result<&BT_CLSOut_T, ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcIO_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcIO_Buffer_T>().as_ref().unwrap();

            Ok( &pmb.cls_out[idx].ctr_out )
        }
    }

    #[cfg(test)]
    pub fn set_ctrl_out(&self, c: &BT_CLSOut_T, idx: usize) -> Result<(), ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcIO_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcIO_Buffer_T>().as_mut().unwrap();
            pmb.cls_out[idx].ctr_out.clone_from(c);
        }

        Ok(())
    }

    pub fn get_drive_out(&self, idx: usize) -> Result<&BT_DriveOut_T, ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcIO_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcIO_Buffer_T>().as_ref().unwrap();

            Ok( &pmb.cls_out[idx].drv_out )
        }
    }

    #[cfg(test)]
    pub fn set_drive_out(&self, d: &BT_DriveOut_T, idx: usize) -> Result<(), ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcIO_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcIO_Buffer_T>().as_mut().unwrap();
            pmb.cls_out[idx].drv_out.clone_from(d);
        }

        Ok(())
    }
}

/// Drop trait for destructor
impl Drop for EcIoShm {
    fn drop(&mut self) {
        let _ = self.close();
    }
}