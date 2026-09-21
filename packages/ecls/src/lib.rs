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
    use crate::ecshm::EcIoShm;

    /// EcPmShm Test, NOTE: place TcParams.dat under directory of Cargo.tml in package.
    #[test]
    fn ecpm_mmap_test() {
        let res = EcPmShm::open(true);
        assert!(res.is_ok(), "Can not open mmap: {:?}", res);
        let ecpm = res.unwrap();

        let res = ecpm.map_as_mut();
        assert!(res.is_ok(), "Can not get EC PM_Buffer: {:?}", res);
        let mbuf = res.unwrap();
        mbuf.ai[..5].copy_from_slice(b"Hello");
        assert_eq!(&mbuf.ai[..5], b"Hello");
    }

    /// EcIoShm Test
    #[test]
    fn ecshm_test() {
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
}