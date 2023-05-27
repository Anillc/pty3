use std::{pin::Pin, task::{Context, Poll, ready}, io, os::fd::{AsRawFd, RawFd}, rc::Rc};
use tokio::io::{unix::AsyncFd, AsyncWrite, AsyncRead, ReadBuf};
use crate::error::{Result, PtyError};

#[derive(Debug)]
pub struct Fd<T: AsRawFd> {
  inner: Rc<AsyncFd<T>>
}

impl<T: AsRawFd> Fd<T> {
    pub fn new(async_fd: Rc<AsyncFd<T>>) -> Result<Fd<T>> {
        let fd = async_fd.get_ref().as_raw_fd();
        Self::set_non_blocking(fd)?;
        Ok(Fd { inner: async_fd })
    }

    fn set_non_blocking(fd: RawFd) -> Result<()> {
        let ret = unsafe {
            libc::fcntl(fd, libc::F_SETFL, libc::fcntl(fd, libc::F_GETFL) | libc::O_NONBLOCK)
        };
        if ret < 0 {
            return Err(PtyError::SyscallFailed(std::io::Error::last_os_error()))
        }
        Ok(())
    }
}

impl<T: AsRawFd> AsyncRead for Fd<T> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut ReadBuf) -> Poll<io::Result<()>> {
        loop {
            let mut guard = ready!(self.inner.poll_read_ready(cx))?;
            let unfilled = buf.initialize_unfilled();
            let result = guard.try_io(|inner| {
                let fd = inner.get_ref().as_raw_fd();
                let ret = unsafe {
                    libc::read(fd, unfilled as *mut _ as *mut _, unfilled.len())
                };
                if ret < 0 {
                    return Err(std::io::Error::last_os_error())
                }
                Ok(ret)
            });
        
            match result {
                Err(_would_block) => continue,
                Ok(Err(err)) => break Poll::Ready(Err(err)),
                Ok(Ok(len)) => {
                    buf.advance(len as usize);
                    break Poll::Ready(Ok(()));
                },
            }
        }
    }
}

impl<T: AsRawFd> AsyncWrite for Fd<T> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        loop {
            let mut guard = ready!(self.inner.poll_write_ready(cx))?;
            let result = guard.try_io(|inner| {
                let fd = inner.get_ref().as_raw_fd();
                let ret = unsafe {
                    libc::write(fd, buf as *const _ as *const _, buf.len())
                };
                if ret < 0 {
                    return Err(std::io::Error::last_os_error())
                }
                Ok(ret as usize)
            });

            match result {
                Err(_would_block) => continue,
                Ok(result) => break Poll::Ready(result),
            }
            
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
