use std::rc::Rc;

use tokio::io::unix::AsyncFd;

use crate::{stream::Fd, error::{Result, PtyError}};

#[derive(Debug)]
pub struct Pty {}

pub enum Open {
    Parent(Fd<libc::c_int>, Fd<libc::c_int>), Child,
}

impl Pty {
    pub fn open() -> Result<Open> {
        let mut master: libc::c_int = 0;
        let ret = unsafe {
            libc::forkpty(&mut master as *mut _, 0 as *mut _, 0 as *mut _, 0 as *mut _)
        };
        if ret < 0 {
            return Err(PtyError::SyscallFailed(std::io::Error::last_os_error()));
        } else if ret != 0 {
            // child
            unsafe {
                let dup1 = libc::dup2(ret, libc::STDIN_FILENO);
                let dup2 = libc::dup2(ret, libc::STDOUT_FILENO);
                let dup3 = libc::dup2(ret, libc::STDERR_FILENO);
                if dup1 != 0 || dup2 != 0 || dup3 != 0 {
                    return Err(PtyError::SyscallFailed(std::io::Error::last_os_error()));
                }
            }
            Ok(Open::Child)
        } else {
            // parent
            let async_fd = Rc::new(AsyncFd::new(master)
                .map_err(|err| PtyError::SyscallFailed(err))?);
            let reader = Fd::new(async_fd.clone())?;
            let writer = Fd::new(async_fd.clone())?;
            Ok(Open::Parent(reader, writer))
        }
    }
}