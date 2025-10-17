//! Cross-platform file descriptor abstraction

#[cfg(unix)]
use std::os::unix::io::RawFd;

#[cfg(windows)]
use std::os::windows::io::RawHandle;

/// Cross-platform file descriptor type
#[derive(Debug, Clone, Copy)]
pub enum PlatformFd {
    #[cfg(unix)]
    Unix(RawFd),
    #[cfg(windows)]
    Windows(RawHandle),
}

unsafe impl Send for PlatformFd {}

impl PlatformFd {
    /// Create a new PlatformFd from a platform-specific raw file descriptor
    #[cfg(unix)]
    pub fn from_raw_fd(fd: RawFd) -> Self {
        PlatformFd::Unix(fd)
    }

    /// Create a new PlatformFd from a platform-specific raw handle
    #[cfg(windows)]
    pub fn from_raw_handle(handle: RawHandle) -> Self {
        PlatformFd::Windows(handle)
    }

    /// Get the raw file descriptor on Unix platforms
    #[cfg(unix)]
    pub fn as_raw_fd(&self) -> RawFd {
        match self {
            PlatformFd::Unix(fd) => *fd,
        }
    }

    /// Get the raw handle on Windows platforms
    #[cfg(windows)]
    pub fn as_raw_handle(&self) -> RawHandle {
        match self {
            PlatformFd::Windows(handle) => *handle,
        }
    }
}

#[cfg(unix)]
impl From<RawFd> for PlatformFd {
    fn from(fd: RawFd) -> Self {
        PlatformFd::Unix(fd)
    }
}

#[cfg(windows)]
impl From<RawHandle> for PlatformFd {
    fn from(handle: RawHandle) -> Self {
        PlatformFd::Windows(handle)
    }
}

#[cfg(unix)]
impl From<PlatformFd> for RawFd {
    fn from(pfd: PlatformFd) -> Self {
        pfd.as_raw_fd()
    }
}

#[cfg(windows)]
impl From<PlatformFd> for RawHandle {
    fn from(pfd: PlatformFd) -> Self {
        pfd.as_raw_handle()
    }
}
