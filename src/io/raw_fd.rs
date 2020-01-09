use crate::io::dispatch::Dispatcher;

use std::future::Future;
use std::io;
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

/// Return a future that reads from a file descriptor once it is ready.
///
/// The dispatcher is sent a read request.
/// Once the request is fulfilled, reading shall begin.
///
/// The file descriptor must be in non-blocking mode.
pub fn read<'a>(d: Dispatcher, fd: RawFd, buf: &'a mut [u8])
    -> impl 'a + Future<Output=io::Result<usize>>
{
    Read{d, fd, buf}
}

struct Read<'a>
{
    d:   Dispatcher,
    fd:  RawFd,
    buf: &'a mut [u8],
}

impl<'a> Future for Read<'a>
{
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>
    {
        let fd = self.fd;

        let ok = unsafe {
            libc::read(
                fd,
                self.buf.as_mut_ptr() as *mut libc::c_void,
                self.buf.len(),
            )
        };

        if ok == -1 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EAGAIN) {
                match self.d.request_read(fd, cx.waker().clone()) {
                    Ok(())       => return Poll::Pending,
                    Err(req_err) => return Poll::Ready(Err(req_err)),
                }
            } else {
                return Poll::Ready(Err(err));
            }
        }

        Poll::Ready(Ok(ok as usize))
    }
}
