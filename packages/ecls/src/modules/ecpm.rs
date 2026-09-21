
//#[path ="./error.rs"] // can find it in current dir.
mod error;
use error::ShmemError; // custom error => Err(ShmemError)

use std::io::Seek;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, AtomicU8};
use libc::{c_void, off_t, PROT_READ, PROT_WRITE, MAP_FAILED, MAP_SHARED, MS_SYNC};


/// CLS Model Control Output ==> Host
#[repr(C)]  // 保持内存布局一致
#[derive(Debug, Copy, Clone)]
pub struct PT_CLSConst {
    pub use_deg: bool,      // Displacement Pattern: 1: Rotary; otherwise Linear.
    pub ccw_dir: bool,      // Counter Clock Wise Direction.
    /** 
     * @details Switch Function:
     *  BIT0: Force Compensation
     *  BIT1: Force Feedback
     *  BIT2: Velocity Filter
     *  BIT3: Velocity First
     *  BIT4: FWD. Force STO
     */
    pub sw_fcw: u16,
    pub torque_limit : u16, // Drive Torque Limit: N/Fmax * 1000.
    pub max_for_out  : f64, // Model Max Force Output Limit.
    pub max_vel_out  : f64, // Model Max Velocity Output Limit.
    pub arm_length   : f64, // Stick or Motor Arm Length, same unit as output.
    pub acu_fwd_pos_r: f64, // ACU ==> FWD. POS Gear Ratio = P_FWD/P_Motor.
    pub acu_fwd_for_r: f64, // ACU ==> FWD. Force Gear Ratio = F_FWD/F_Motor.
    pub afw_fwd_ratio: f64, // AFW.==> FWD. Gear Ratio, Cable Scale: x2/x1 = P, Stick Scale = 1.
    pub for_out_scale: f64, // Force (N) ==> Tcmd: = 1000/200(rating).
    pub vel_out_scale: f64, // RPM ==> Vcmd.
    pub pos_out_sacle: f64, // DEG ==> Pcmd.
    pub for_in_scale : f64, // Input Torque ==> Force (N): Kistler Amplifier Scaling Range / 32767.
    pub vel_in_scale : f64, // Input VEL ==> deg/s:= (360/60) / 1000.
    pub pos_in_scale : f64, // Input POS (revolution) ==> Degree:= 360 / (2^20).}
}

/// CLS Model Parameters
#[allow(non_snake_case)]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PT_CLSModel {
    pub bA : f64,   // AFT. damping
    pub bF : f64,   // FWD. damping
    pub bH : f64,   // Stick Column Linkage damping
    pub bS : f64,   // Cable-Stick damping
    pub kS : f64,   // Cable-Stick stiffness
    pub kH : f64,   // Stick Column Linkage stiffness
    pub kHV: f64,   // Stick Column Linkage damping-strength
    pub kSV: f64,   // Cable-Stick damping-strength
    pub kQ : f64,   // Force Feedback gain: its result must be the same sign as Ku output.
    pub kU : f64,   // Force Model Output gain: its result should be same as the force feedback.
    pub mA : f64,   // AFT. mass
    pub mF : f64,   // FWD. mass
    pub Vbrk:f64,   // Start velocity zone for friction.
    pub Xbrk:f64,   // Start position zone for friction.
    pub FcA: f64,   // AFW. Coulomb friction
    pub FcF: f64,   // FWD. Coulomb friction
    pub bFA: f64,   // FWD. & AFT. Start Force / Static Friction damping.
    pub dzF: f64,   // FWD. dead zone
    pub NFC_P: f64, // Force Following PID Controller: P = 4
    pub NFC_I: f64, // Force Following PID Controller: I < 1E-3
    pub NFC_D: f64, // Force Following PID Controller: D = 1
    pub NFC_N: f64, // Force Following PID Controller: N = 0.1, Low-pass Filter.
}

/// CLS Controller Parameters
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PT_CLSParam {
    pub auto_homing: bool,   // Auto Homing Enable
    pub vel_pos   : f64,     // Position Velocity: RPM
    pub jagment   : f64,     // Jag Move Degree
    pub jag_lim_p : f64,     // Positive Limited Position: Degree
    pub jag_lim_n : f64,     // Negative Limited Position: Degree
    pub pos_home  : f64,     // Home Position: Degree (NOT Counts)
    pub for_offset: f64,     // Input Force Offset
    pub for_zero  : f64,     // Zero Force Offset
    pub f0_a_off  : f64,     // Aero Force Offset
    pub pos_offset: f64,     // Input Position Offset
    pub p0_trim   : f64,     // Init Trim Position
    pub l0_trim   : f64,     // Trim Limit such as 10 deg
    pub l0_trav_a : f64,     // Travel Above Limit
    pub l0_trav_b : f64,     // Travel Below Limit
    pub shaker_a  : f64,     // Shaker Amplitude
    pub shaker_f  : f64,     // Shaker Frequency
    pub vt1: f64,            // Velocity for Transition from Other to Force
    pub vt2: f64,            // Velocity for Transition 2
}


/// 等价于 C 语言中的 `struct PT_CLSpring`
/// 将 `real_T` 映射为 Rust 的 `f64`
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PT_CLSpring {
    pub k0: f64,    // BreakoutGradient [Nm/deg]
    pub k1: f64,    // FeelSpringSlope1 [Nm/deg]
    pub k2: f64,    // FeelSpringSlope2 [Nm/deg]
    pub k3: f64,    // FeelSpringSlope3 [Nm/deg]
    pub k4: f64,    // FeelSpringSlope4 [Nm/deg]
    pub k5: f64,    // FeelSpringSlope5 [Nm/deg]
    pub x0: f64,    // BreakoutLevel [Nm]
    pub x1: f64,    // FeelSpringBP1 [deg]
    pub x2: f64,    // FeelSpringBP2 [deg]
    pub x3: f64,    // FeelSpringBP3 [deg], or Forward Position Limit Below
    pub x4: f64,    // FeelSpringBP4 [deg], or Forward Position Limit Above
    pub x5: f64,    // FeelSpringBP5 [deg], or Forward Force Limit Position Below
    pub x6: f64,    // FeelSpringBP6 [deg], or Forward Force Limit Position Above
    pub ke: f64,    // Uniform Scale for K5-Spring
}


#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PT_CLSTest {
    pub km: f64,    // Test Spring: 1/m, reserved
    pub ks: f64,    // Spring Test: Spring Stiffness
    pub kp: f64,    // Test Spring PID: Kp, reserved
    pub ka: f64,    // Test Spring PID: Ka, reserved
    pub kq: f64,    // Test Spring Force feedback: Kq, reserved
    pub bs: f64,    // Spring Test: negtive damping, 0.02 to overcome the frictions == bsf
    pub ba: f64,    // T5-section spring: damping
    pub ksf:f64,    // Free Test: stiffness
    pub bsf:f64,    // Free Test: damping, -0.02 to overcome the frictions
    pub dl: f64,    // damping limit: 6.5 N maybe good.
    pub pl: f64,    // Free Test: Position Limit, 40 degree may be good.
}

/// Parameters (default storage)
#[allow(non_snake_case)]
#[repr(C)] // 保证与 C 内存布局一致
#[derive(Debug, Clone, Copy)]
pub struct P_TcLCS_T {
    pub CLSConsts: PT_CLSConst, // 对应 C 的 PT_CLSConst
    pub CLSModel:  PT_CLSModel, // 对应 C 的 PT_CLSModel
    pub CLSParam:  PT_CLSParam, // 对应 C 的 PT_CLSParam
    pub CLS5K:     PT_CLSpring, // 对应 C 的 PT_CLSpring
    pub TestMDL:   PT_CLSTest,  // 对应 C 的 PT_CLSTest
    pub XT: [f64; 21],          // 对应 C 的 real_T XT[21]
    pub YT: [f64; 21],          // 对应 C 的 real_T YT[21]
}

impl Default for P_TcLCS_T {
    fn default() -> Self {
        Self {
            CLSConsts: PT_CLSConst {	// INOVANCE DRIVE Params
                use_deg      : true,	// KUSEDEG
                ccw_dir      : true,	// CCWDIR
                sw_fcw       : 2,   	// SWFCW
                torque_limit : 1000u16,	// KFLMT
                max_for_out  : 320.0,	// KFmax
                max_vel_out  : 360.0,	// KVmax
                arm_length   : 10.0,	// Larm 
                acu_fwd_pos_r: 0.1,  	// KPR
                acu_fwd_for_r: 1.0,  	// KFR
                afw_fwd_ratio: 1.0, 	// KX2P
                for_out_scale: 4.0,  	// KForceTo: sAI1[-1000,1000] x4 (N)
                vel_out_scale: 23301.688888889,	// KVelTo
                pos_out_sacle: 23301.688888889,	// KPosTo
                for_in_scale :-0.25,        	// KF2N: NSensor ==> FwdForce
                vel_in_scale : 360.0/8388608.0,	// KV2DPS
                pos_in_scale : 360.0/8388608.0,	// KP2DEG
            },
            CLSModel: PT_CLSModel {
                bA  : 0.0,
                bF  : 0.0,
                bH  : 1.0,
                bS  : 5.3,
                kS  : 1000.0,
                kH  : 100.0,
                kHV : 1.0, 
                kSV : 1.0,
                kQ  : 1.0,
                kU  : 0.45,
                mA  : 0.02,
                mF  : 0.01,
                Vbrk: 4.0,
                Xbrk: 1.0,
                FcA : 0.0,
                FcF : 0.0,
                bFA : 0.0,
                dzF : 0.0,
                NFC_P: 1.0,   
                NFC_I: 0.0,
                NFC_D: 0.3,
                NFC_N: 0.0,
            },
            CLSParam: PT_CLSParam {
                auto_homing: true,	// AutoHoming
                vel_pos   : 1.0,	// VPos
                jagment   : 6.0,	// Jagment
                jag_lim_p : 18.0,	// JagmentP
                jag_lim_n :-18.0,	// JagmentN
                pos_home  : 0.0,	// P0Home
                for_offset: 0.0,	// Foffset
                for_zero  : 0.0,	// Fzero
                f0_a_off  : 0.0,	// F0Aoff
                pos_offset: 0.0,	// Poffset
                p0_trim   : 0.0,	// P0Trim
                l0_trim   : 10.0,	// L0Trim
                l0_trav_a : 15.0,	// L0TravA
                l0_trav_b :-15.0,	// L0TravB
                shaker_a  : 0.0,	// ShakerA
                shaker_f  : 30.0,	// ShakerF
                vt1       : 0.5,	// VT1
                vt2       : 1.0,	// VT2
            },
            CLS5K: PT_CLSpring { // CLS5K: SpringK3B
                k0:  0.0,	// K0
                k1:  1.0,	// K1
                k2:  0.0,	// K2
                k3:  0.0,	// K3
                k4:  0.0,	// K4
                k5:  5.0,	// K5
                x0:  0.0,	// X0
                x1:  4.0,	// X1
                x2:  8.0,	// X2
                x3:  12.0,	// X3
                x4:  16.0,	// X4
                x5:  20.0,	// X5
                x6: -20.0,	// X6
                ke:  1.0,	// Ke
            },
            TestMDL: PT_CLSTest {
                km :  1.0,     // Km
                ks :  10.0,    // Ks
                kp :  5.0,     // Kp
                ka : -1.0,     // Ka
                kq :  0.0,     // Kq
                bs :  0.0,     // bs
                ba :  0.0,     // bA
                ksf:  2000.0,  // Ksf
                bsf:  1.1,     // bsf
                dl :  32.5,    // DL
                pl :  15.0,    // PL
            },
            XT: [ -50.0, -45.0, -40.0, -35.0, -30.0, -25.0, -20.0, -15.0, -10.0, -5.0,
                    0.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0 ],
            YT: [0.0; 21],
        }
    }
}

///
/// @brief TC CLS Parameters Buffer [1024 bytes]
/// @note use initializer list to ensure all members are initialized to zero.
///   ex. EcPM_Buffer_T buf = {};
///   -std=c++20 -O1 may optimize: "rep stosq"
/// 
#[allow(non_snake_case)]
#[repr(C, align(8))] // 保证与 alignas(8) 一致
#[derive(Debug)]
pub struct EcPM_Buffer_T {
    dwSize: u32, // readonly, sizeof(EcPM_Buffer_T)
    nChans: u16, // readonly, channel number

    pub pm_updated: AtomicBool, // update flag
    pub pm_splock : AtomicU8,   // spin lock (atomic_flag 等价为 AtomicU8/AtomicBool)
    pub params: [P_TcLCS_T; Self::K_SIZE], // 960 * k bytes
    pub pm_writing: AtomicBool, // read/write flag
    /// 保留 7 字节（Rust 中用 [u8; 7]）
    pub reserved: [u8; 7],
    /// 对齐到 8 字节的 UUID 区域
    pub ai: [u8; 32],
    /// Magic Tag and Version
    szTag: [u8; 16],    // readonly, 固定长度字符串: "TcPM_Buffer 2.0\0"
}

impl EcPM_Buffer_T {
    const K_SIZE: usize = 1; // 通道数量: 1~10

    /// 创建一个默认初始化的缓冲区
    pub fn new() -> Self {
        let buf = EcPM_Buffer_T {
            dwSize: size_of::<EcPM_Buffer_T>() as u32,
            nChans: Self::K_SIZE as u16,
            pm_updated: AtomicBool::new(false),
            pm_splock : AtomicU8::new(0),
            params: [P_TcLCS_T::default(); Self::K_SIZE],
            pm_writing: AtomicBool::new(false),
            reserved: [0; 7],
            ai: [0; 32],
            szTag: *b"TcPM_Buffer 1.0\0", // b"TcPM_Buffer 1.0\0" ==> &[u8; 16] slice
        };

        buf
    }

    /// 获取通道数
    pub fn get_chans_num(&self) -> usize {
        self.nChans as usize
    }

    /// 获取 Tag
    pub fn get_tag(&self) -> &str {
        str::from_utf8(&self.szTag).unwrap_or("Error")
    }

    /// size_of
    pub fn size_of() -> usize {
        std::mem::size_of::<Self>()
    }
}


#[allow(unused)]
#[derive(Debug)]
pub struct EcPmShm {
    mct: u32,   // Lock timeout in counts.
    mfd: File,  // Shared memory file descriptor
    msz: usize, // Shared memory size in bytes
    mpbuf: *mut EcPM_Buffer_T,  // Shared memory address
}

impl EcPmShm {
    const LOCK_TIME_OUT: u32 = 10;
    #[cfg(target_os = "linux")]
    const SHM_PATH: &'static str = "/usr/local/etc/TcParams.dat";
    #[cfg(not(target_os = "linux"))]
    const SHM_PATH: &'static str = "TcParams.dat";

    pub fn create(size: usize) -> Result<Self, std::io::Error> {
        let mut f = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .open(Self::SHM_PATH)?;
        let sz = if size >= size_of::<EcPM_Buffer_T>() {
                size
            } else {
                size_of::<EcPM_Buffer_T>()
            };
        f.seek(std::io::SeekFrom::Start(0))?;
        f.set_len(sz as u64)?;
        let fd = f.as_raw_fd();

        unsafe {
            let addr = libc::mmap(
                std::ptr::null_mut(),
                sz,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd,
                0 as off_t,
            );

            if addr == MAP_FAILED {
                return Err(std::io::Error::last_os_error());
            }

            let buf = EcPM_Buffer_T::new();
            let ptr = &buf as *const EcPM_Buffer_T as *const c_void;
            addr.copy_from(ptr, EcPM_Buffer_T::size_of());

            Ok(EcPmShm{
                mct: Self::LOCK_TIME_OUT,
                mfd: f,
                msz: sz,
                mpbuf: addr as *mut EcPM_Buffer_T,
            })
        }
    }

    pub fn open(writable: bool) -> Result<Self, std::io::Error> {
        let file = std::fs::OpenOptions::new()
                        .read(true)
                        .write(writable)
 //                     .create(true)
                        .open(Self::SHM_PATH)?;
        let meta = file.metadata()?;
        let sz = meta.len();
        if sz < EcPM_Buffer_T::size_of() as u64 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                 "File is not enough"));
        }

        let mut mflag = PROT_READ;
        if writable {
            mflag |=  PROT_WRITE;
        }
        unsafe {
            let addr = libc::mmap(
                std::ptr::null_mut(),
                sz as usize,
                mflag,
                MAP_SHARED,
                file.as_raw_fd(),
                0 as off_t,
            );

            if addr == MAP_FAILED {
                return Err(std::io::Error::last_os_error());
            }

            Ok(EcPmShm{
                mct: Self::LOCK_TIME_OUT,
                mfd: file,
                msz: sz as usize,
                mpbuf: addr as *mut EcPM_Buffer_T,
            })
        }
    }

    pub fn map_as_readonly(&self) -> Result<&EcPM_Buffer_T, ShmemError> {
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        }
        if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { Ok(
            self.mpbuf.cast::<EcPM_Buffer_T>().as_ref().unwrap() )
        }
    }

    pub fn map_as_mut(&self) -> Result<&mut EcPM_Buffer_T, ShmemError> {
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        }
        if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { Ok(
            self.mpbuf.cast::<EcPM_Buffer_T>().as_mut().unwrap() )
        }
    }

/** @brief *const T, and *mut T
 *  @example: value type
 *    let my_num: i32 = 10;       let my_num_ptr: *const i32 = &my_num;
 *    let mut my_speed: i32 = 88; let my_speed_ptr: *mut i32 = &mut my_speed;
 *  @example: struct type
 *    struct S {
 *        aligned: u8,
 *        unaligned: u32,
 *    }
 *    let s: *const S = S::default();     let ps = &raw const s;
 *    let x: *mut   S = S::default();     let px = &raw mut   x;
 */

    pub fn bytes_as_readonly(&self) -> Result<&[u8], ShmemError> {
        if self.mpbuf.is_null() ||
           self.msz < size_of::<EcPM_Buffer_T>() {
            return Err(ShmemError::MapSizeZero);
        }

        unsafe { Ok(
            std::slice::from_raw_parts(self.mpbuf as *const u8, self.msz) )
        }
    }

    pub fn bytes_as_mut(&self) -> Result<&mut[u8], ShmemError> {
        if self.mpbuf.is_null() ||
           self.msz < size_of::<EcPM_Buffer_T>() {
            return Err(ShmemError::MapSizeZero);
        }

        unsafe { Ok(
            std::slice::from_raw_parts_mut(self.mpbuf as *mut u8, self.msz) )
        }
    }

    ///////////////////////////////////////////////////////////////////////////

    pub fn get_consts(&self, idx: usize) -> Result<&PT_CLSConst, ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_ref().unwrap();

            Ok( &pmb.params[idx].CLSConsts )
        }
    }

    pub fn set_consts(&self, c: &PT_CLSConst, idx: usize) -> Result<(), ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_mut().unwrap();
            pmb.params[idx].CLSConsts.clone_from(c);

            Ok(())
        }
    }

    pub fn get_model(&self, idx: usize) -> Result<&PT_CLSModel, ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_ref().unwrap();

            Ok( &pmb.params[idx].CLSModel )
        }
    }

    pub fn set_model(&self, m: &PT_CLSModel, idx: usize) -> Result<(), ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_mut().unwrap();
            pmb.params[idx].CLSModel.clone_from(m);

            Ok(())
        }
    }

    pub fn get_param(&self, idx: usize) -> Result<&PT_CLSParam, ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_ref().unwrap();

            Ok( &pmb.params[idx].CLSParam )
        }
    }

    pub fn set_param(&self, p: &PT_CLSParam, idx: usize) -> Result<(), ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_mut().unwrap();
            pmb.params[idx].CLSParam.clone_from(p);

            Ok(())
        }
    }

    pub fn get_s5k(&self, idx: usize) -> Result<&PT_CLSpring, ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_ref().unwrap();

            Ok( &pmb.params[idx].CLS5K )
        }
    }

    pub fn set_s5k(&self, k: &PT_CLSpring, idx: usize) -> Result<(), ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_mut().unwrap();
            pmb.params[idx].CLS5K.clone_from(k);

            Ok(())
        }
    }

    pub fn get_test_model(&self, idx: usize) -> Result<&PT_CLSTest, ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_ref().unwrap();

            Ok( &pmb.params[idx].TestMDL )
        }
    }

    pub fn set_test_model(&self, m: &PT_CLSTest, idx: usize) -> Result<(), ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_mut().unwrap();
            pmb.params[idx].TestMDL.clone_from(m);

            Ok(())
        }
    }

    pub fn get_xt(&self, idx: usize) -> Result<&[f64;21], ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_ref().unwrap();

            Ok( &pmb.params[idx].XT )
        }
    }

    pub fn set_xt(&self, x: &[f64;21], idx: usize) -> Result<(), ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_mut().unwrap();
            pmb.params[idx].XT.clone_from(x);

            Ok(())
        }
    }

    pub fn get_yt(&self, idx: usize) -> Result<&[f64;21], ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_ref().unwrap();

            Ok( &pmb.params[idx].YT )
        }
    }

    pub fn set_yt(&self, y: &[f64;21], idx: usize) -> Result<(), ShmemError>{
        if idx > 0 { // now only one channel
            return Err(ShmemError::IndexOverError);
        }
        if self.mpbuf.is_null() {
            return Err(ShmemError::MapSizeZero);
        } else if self.mpbuf.align_offset(std::mem::align_of::<EcPM_Buffer_T>()) != 0 {
            return Err(ShmemError::MapSizeError);
        }

        unsafe { 
            let pmb = self.mpbuf.cast::<EcPM_Buffer_T>().as_mut().unwrap();
            pmb.params[idx].YT.clone_from(y);

            Ok(())
        }
    }

    ///////////////////////////////////////////////////////////////////////////

    pub fn close(&mut self) -> Result<(), std::io::Error> {
        let mut m = 0;
        unsafe {
            if !self.mpbuf.is_null() {
                libc::msync(self.mpbuf as *mut c_void, self.msz, MS_SYNC);
                m = libc::munmap(self.mpbuf as *mut c_void, self.msz);
                self.mpbuf = std::ptr::null_mut();
            }
        }
        if m == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

/// Drop trait for destructor
impl Drop for EcPmShm {
    fn drop(&mut self) {
        let _ = self.close();
    }
}