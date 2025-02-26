pub type RawFd = i32;
pub type socklen_t = u32;
pub type off64_t = i64;

pub trait AsRawFd {
    fn as_raw_fd(&self) -> RawFd;
}

pub trait FromRawFd {
    unsafe fn from_raw_fd(fd: RawFd) -> Self;
}

pub trait IntoRawFd {
    fn into_raw_fd(self) -> RawFd;
}

