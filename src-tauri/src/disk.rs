use std::ffi::CString;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DiskUsage {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

pub fn usage() -> Option<DiskUsage> {
    let root = CString::new("/").ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(root.as_ptr(), &mut stat) } != 0 {
        return None;
    }
    let frsize = stat.f_frsize as u64;
    Some(DiskUsage {
        total_bytes: stat.f_blocks as u64 * frsize,
        available_bytes: stat.f_bavail as u64 * frsize,
    })
}
