// Declare modules
#[path ="modules/server.rs"]
pub mod server;

#[path ="modules/client.rs"]
pub mod client;

#[path ="modules/ecpm.rs"]
pub mod ecpm;

#[path ="modules/ecshm.rs"]
pub mod ecshm;

#[path ="modules/error.rs"]
mod error;

pub use error::*;   // pub ==> export ShmemError


#[allow(unused_macros)]
mod log2 { // pub ==> export, also outside crate
    macro_rules! trace (($($tt:tt)*) => {{}});
    macro_rules! debug (($($tt:tt)*) => {{}});
    macro_rules! info  (($($tt:tt)*) => {{}});
    macro_rules! warn  (($($tt:tt)*) => {{}});
    macro_rules! error (($($tt:tt)*) => {{}});
//  pub(crate) use {debug, trace};
}

pub mod crcs {
    /// Fast Lookup Table Cyclic Redundancy Check, 8-bit
    /// 
    /// Precomputed CRC-8 lookup table for polynomial 0x07 (CRC-8-ATM)
    const CRC8_TABLE: [u8; 256] = {
        let mut table = [0u8; 256];
        let poly = 0x07u8;  // TODO: change the polynomial: 0x07
        let mut i = 0;
        while i < 256 {
            let mut crc = i as u8;
            let mut j = 0;
            while j < 8 {
                if crc & 0x80 != 0 {
                    crc = (crc << 1) ^ poly;
                } else {
                    crc <<= 1;
                }
                j += 1;
            }
            table[i] = crc;
            i += 1;
        }
        table
    };

    /// Fast Lookup Table Cyclic Redundancy Check, 16-bit
    /// 
    /// Precomputed CRC-16 lookup table for polynomial 0x8005 (CRC-16-IBM)
    /// Polynomial = x¹⁶ + x¹⁵ + x² + 1 (0x8005 LSB)
    /// LSB: H----------L [<<]; MSB: L-----------H [>>]
    const CRC16_TABLE: [u16; 256] = {
        let mut table = [0u16; 256];
        // TODO: change the polynomial: 0x8005: LSB
        //       or 0xA001 (CRC‑16‑ANSI) for Reflect: MSB
        let poly = 0x8005u16;
        let mut i = 0;
        while i < 256 {
            let mut crc = (i as u16) << 8;
            let mut j = 0;
            while j < 8 {
                if crc & 0x8000 != 0 {
                    crc = (crc << 1) ^ poly;
                } else {
                    crc <<= 1;
                }
                j += 1;
            }
            table[i] = crc;
            i += 1;
        }
        table
    };

    /// Cyclic Redundancy Check, 8-bit
    /// 
    /// a lightweight error-detection algorithm often used in embedded systems
    /// and data transmission to ensure data integrity
    /// Polynomial = x⁸ + x² + x + 1 (0x07)
    #[allow(dead_code)]
    pub fn crc8_atm(data: &[u8], polynomial: u8, init: u8) -> u8 {
        let mut crc = init;
        for &byte in data {
            crc ^= byte;
            for _ in 0..8 {
                if crc & 0x80 != 0 {
                    crc = (crc << 1) ^ polynomial;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }

    /// Compute CRC-8 using lookup table
    /// 
    /// init: 0, 0xFF, ...
    #[allow(dead_code)]
    pub fn crc8_lookup(data: &[u8], init: u8) -> u8 {
        let mut crc = init;
        for &byte in data {
            crc = CRC8_TABLE[(crc ^ byte) as usize];
        }
        crc
    }

    /// Compute CRC-16 using lookup table
    pub fn crc16_lookup(data: &[u8], init: u16) -> u16 {
        let mut crc = init;
        for &byte in data {
            let index = ((crc >> 8) ^ byte as u16) as usize;
            crc = (crc << 8) ^ CRC16_TABLE[index];
        }
        crc
    }

    /// Computes CRC-16-IBM (CRC-16-ANSI/ARC) manually, bit-by-bit.
    /// Polynomial = x¹⁶ + x¹⁵ + x² + 1 (0x8005 LSB)
    /// LSB: H----------L [<<]; MSB: L-----------H [>>]
    /// 
    /// Polynomial: 0xA001 (reversed form of 0x8005)
    /// Initial value: 0xFFFF
    /// Input  reflected: Yes
    /// Output reflected: Yes
    /// Final XOR: 0x0000
    pub fn crc16_ibm_reversed(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;  // TODO: may be 0x0000
        let poly: u16 = 0xA001;     // reversed polynomial

        for &byte in data {
            crc ^= byte as u16;
            for _ in 0..8 {
                if crc & 0x0001 != 0 {
                    crc = (crc >> 1) ^ poly;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
    }
}

//pub use crate::log2::*; // ==> Internal, not outside crate

#[cfg(test)]
mod tests {
    use crate::ecpm::EcPmShm;
    use crate::ecshm::*;


    /// EcPmShm Test, NOTE: place TcParams.dat under directory of Cargo.tml in package.
    #[test]
    fn ecpm_mmap_test() {
        let ecpm = if let Ok(ec) = EcPmShm::open(true) {
            ec
        } else {
            EcPmShm::create(0).expect("Failed to create share memory file")
        };

        let res = ecpm.map_as_mut();
        assert!(res.is_ok(), "Can not get EC PM_Buffer: {:?}", res);
        let mbuf = res.unwrap();
        mbuf.ai[..5].copy_from_slice(b"Hello");
        assert_eq!(&mbuf.ai[..5], b"Hello");
    }

    /// EcIoShm Test
    #[test]
    fn ecshm_shm_test() {
        // EcIoShm::delete();
        let ecshm = if let Ok(ec) =  EcIoShm::open(true) {
            ec
        } else {
            EcIoShm::create(0).expect("Failed to create share memory")
        };

        let cin = ecshm.get_ctrl_in(0).expect("can not get cls_in");
        assert_eq!(cin.trav_a, 15.0);
        assert_eq!(cin.trav_b,-15.0);
    }

    #[test]
    fn crcs_test() {
        let bytes = [0x01, 0x03, 0x00, 0x00, 0x00, 0x0A];
        let sum = crate::crcs::crc16_ibm_reversed(&bytes[..]);
        assert_eq!(sum, 0xCDC5, "CRC NOT Correct");
    }
}