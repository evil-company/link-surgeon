use crate::IDEF;
use std::error::Error;

pub type BoxErr = Box<dyn Error>;
pub type BoxErrSendSync = Box<dyn Error + Send + Sync>;

pub fn log_if_err<E: std::fmt::Display>(what: &str, r: Result<(), E>) {
    if let Err(e) = r {
        log::error!("{IDEF}: {what} failed: {e}");
    }
}
