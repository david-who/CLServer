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

    /// Struct Test
    #[test]
    fn struct_test() {
        let mut to_host = BT_CLS_TO_HOST_T::new();
        let mut fr_host = BT_CLS_FROM_HOST_T::new();

        to_host.set_sum(0x10E);
        fr_host.set_fault(0x1AA);
        assert_eq!(to_host.get_sum(), 0x0E);
        assert_eq!(fr_host.get_fault(), 0xAA);
    }
}