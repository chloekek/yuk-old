use std::io;
use std::os::unix::io::RawFd;

pub use libc::EPOLL_CLOEXEC;

pub use libc::EPOLL_CTL_ADD;
pub use libc::EPOLL_CTL_MOD;
pub use libc::EPOLL_CTL_DEL;

// The libc crate defines these constants of type i32,
// but the epoll API expects them to be passed as u32,
// so we redefine them here with the appropriate type.
pub const EPOLLET:      u32 = libc::EPOLLET      as u32;
pub const EPOLLIN:      u32 = libc::EPOLLIN      as u32;
pub const EPOLLOUT:     u32 = libc::EPOLLOUT     as u32;
pub const EPOLLONESHOT: u32 = libc::EPOLLONESHOT as u32;

/// Owned epoll(7) instance.
///
/// Dropping the object closes the epoll(7) instances.
/// epoll(7)-related operations are presented sans the “epoll_” prefix.
/// These methods are safe, unlike their libc counterparts.
pub struct Epoll
{
    raw: RawFd,
}

pub type EpollEvent =
    libc::epoll_event;

impl Epoll
{
    pub fn create1(flags: i32) -> io::Result<Self>
    {
        let ok = unsafe { libc::epoll_create1(flags) };
        if ok == -1 { return Err(io::Error::last_os_error()) }
        Ok(Self{raw: ok})
    }

    pub fn ctl(&self, op: i32, fd: RawFd, event: &mut EpollEvent)
        -> io::Result<()>
    {
        let ok = unsafe { libc::epoll_ctl(self.raw, op, fd, event) };
        if ok == -1 { return Err(io::Error::last_os_error()) }
        Ok(())
    }

    pub fn wait(&self, events: &mut [EpollEvent], timeout: i32)
        -> io::Result<usize>
    {
        // epoll_wait expects an int, but we must make sure that we do not
        // overflow an int when converting the slice size to it.
        let max_len = libc::c_int::max_value() as usize;
        let len = events.len().min(max_len) as libc::c_int;

        let ok = unsafe {
            libc::epoll_wait(self.raw, events.as_mut_ptr(), len, timeout)
        };
        if ok == -1 { return Err(io::Error::last_os_error()) }
        Ok(ok as usize)
    }
}

impl Drop for Epoll
{
    fn drop(&mut self)
    {
        unsafe { libc::close(self.raw) };
    }
}
