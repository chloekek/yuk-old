use crate::io::epoll::EPOLLET;
use crate::io::epoll::EPOLLIN;
use crate::io::epoll::EPOLLONESHOT;
use crate::io::epoll::EPOLLOUT;
use crate::io::epoll::EPOLL_CLOEXEC;
use crate::io::epoll::EPOLL_CTL_ADD;
use crate::io::epoll::EPOLL_CTL_MOD;
use crate::io::epoll::Epoll;
use crate::io::epoll::EpollEvent;

use std::collections::HashMap;
use std::collections::VecDeque;
use std::io;
use std::mem;
use std::os::unix::io::RawFd;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Waker;

/// A dispatcher keeps track of pending I/O requests.
///
/// After _subscribing_ the dispatcher to a file descriptor,
/// you can _request_ reads and writes from it.
/// These requests will be fulfilled later;
/// when the corresponding operations can be performed without blocking.
pub struct Dispatcher
{
    raw: Arc<Mutex<RawDispatcher>>,
}

struct RawDispatcher
{
    /// For each event, the u64 field is set to
    /// the file descriptor that the event is associated with.
    /// This allows us to relate events back to file descriptor.
    epoll: Epoll,

    /// Pending read requests.
    read_requests: HashMap<RawFd, VecDeque<Waker>>,

    /// Pending write requests.
    write_requests: HashMap<RawFd, VecDeque<Waker>>,
}

impl RawDispatcher
{
    /// Create a dispatcher with a new epoll(7) instance and no wakers.
    fn new(epoll_cloexec: bool) -> io::Result<Self>
    {
        let epoll_flags    = if epoll_cloexec { EPOLL_CLOEXEC } else { 0 };
        let epoll          = Epoll::create1(epoll_flags)?;
        let read_requests  = HashMap::new();
        let write_requests = HashMap::new();
        Ok(Self{epoll, read_requests, write_requests})
    }

    /// Add a new file descriptor to the dispatcher.
    ///
    /// This operation must be performed before requesting a read or a write,
    /// otherwise the latter operations will fail.
    fn subscribe(&mut self, fd: RawFd) -> io::Result<()>
    {
        let mut event = EpollEvent{events: 0, u64: fd as u64};
        self.epoll.ctl(EPOLL_CTL_ADD, fd, &mut event)?;
        Ok(())
    }

    /// Modify the epoll(7) event for the file descriptor
    /// using the current pending requests.
    fn resubscribe(&mut self, fd: RawFd) -> io::Result<()>
    {
        let mut events = 0;

        // We expose an on-demand API, where requests are made for every read
        // call. Clean up the state automatically using this flag.
        events |= EPOLLONESHOT;

        // We rely on EAGAIN being reported for asynchronous I/O, so we can use
        // the more efficient edge-triggered epoll(7) interface.
        events |= EPOLLET;

        // Add the appropriate flags for the pending requests.
        if self.read_requests .get(&fd).iter().any(|q| !q.is_empty())
            { events |= EPOLLIN  }
        if self.write_requests.get(&fd).iter().any(|q| !q.is_empty())
            { events |= EPOLLOUT }

        // Tell epoll(7) about the new requests.
        let mut event = EpollEvent{events, u64: fd as u64};
        self.epoll.ctl(EPOLL_CTL_MOD, fd, &mut event)?;

        Ok(())
    }

    /// Request a read on the given file descriptor.
    ///
    /// Once data becomes available for reading,
    /// the given waker will be woken.
    fn request_read(&mut self, fd: RawFd, waker: Waker) -> io::Result<()>
    {
        // Add the waker to the queue, creating the queue if necessary.
        self.read_requests.entry(fd)
            .or_insert_with(|| VecDeque::with_capacity(1))
            .push_back(waker);

        // Apply the change to the epoll(7) instances.
        // TODO: If this fails, rollback the queue change.
        self.resubscribe(fd).unwrap_or_else(|_| unimplemented!());

        Ok(())
    }

    /// Request a write on the given file descriptor.
    ///
    /// Once data becomes available for writing,
    /// the given waker will be woken.
    fn request_write(&mut self, fd: RawFd, waker: Waker) -> io::Result<()>
    {
        // Add the waker to the queue, creating the queue if necessary.
        self.write_requests.entry(fd)
            .or_insert_with(|| VecDeque::with_capacity(1))
            .push_back(waker);

        // Apply the change to the epoll(7) instances.
        // TODO: If this fails, rollback the queue change.
        self.resubscribe(fd).unwrap_or_else(|_| unimplemented!());

        Ok(())
    }

    /// Wait for I/O operations to be ready,
    /// then wake the appropriate wakers.
    ///
    /// Only one thread can poll at a time.
    #[allow(deprecated)]
    fn poll(&mut self) -> io::Result<()>
    {
        let mut events: [_; 1024] = unsafe { mem::uninitialized() };
        let ready_count  = self.epoll.wait(&mut events, /* timeout */ 0)?;
        let ready_events = events.iter().take(ready_count);

        for event in ready_events {
            let fd = event.u64 as RawFd;

            if event.events & EPOLLIN != 0 {
                let logic_err = "EPOLLIN without corresponding read request.";
                let queue = self.read_requests.get_mut(&fd).expect(logic_err);
                let waker = queue.pop_front().expect(logic_err);
                waker.wake();
            }

            if event.events & EPOLLOUT != 0 {
                let logic_err = "EPOLLOUT without corresponding write request.";
                let queue = self.write_requests.get_mut(&fd).expect(logic_err);
                let waker = queue.pop_front().expect(logic_err);
                waker.wake();
            }

            self.resubscribe(fd)?;
        }

        Ok(())
    }
}
