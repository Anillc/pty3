use std::sync::Arc;
use tokio::io::unix::AsyncFd;
use crate::{stream::Fd, error::{Result, PtyError}};

pub enum Open {
    Parent(Fd<libc::c_int>, Fd<libc::c_int>), Child,
}

pub fn open() -> Result<Open> {
    let mut master: libc::c_int = 0;
    let ret = unsafe {
        libc::forkpty(&mut master as *mut _, 0 as *mut _, 0 as *mut _, 0 as *mut _)
    };
    if ret < 0 {
        return Err(PtyError::SyscallFailed(std::io::Error::last_os_error()));
    } else if ret == 0 {
        // child
        Ok(Open::Child)
    } else {
        // parent
        let async_fd = Arc::new(AsyncFd::new(master)
            .map_err(|err| PtyError::SyscallFailed(err))?);
        let reader = Fd::new(async_fd.clone())?;
        let writer = Fd::new(async_fd.clone())?;
        Ok(Open::Parent(reader, writer))
    }
}

#[cfg(test)]
mod tests {
    use std::{process::Command, os::unix::process::CommandExt};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, stdin, AsyncBufReadExt, stdout};
    use super::{Open, open};

    #[tokio::test]
    async fn test_pty() {
        let mut stdout = stdout();
        let open = open().unwrap();
        match open {
            Open::Child => Command::new("bash").exec(),
            Open::Parent(mut reader, mut writer) => {
                tokio::spawn(async move {
                    let mut stdin = BufReader::new(stdin());
                    loop {
                        let mut line = String::new();
                        stdin.read_line(&mut line).await.unwrap();
                        writer.write_all(line.as_bytes()).await.unwrap();
                    }
                });
                let mut input: [u8; 1024] = unsafe { std::mem::zeroed() };
                loop {
                    let len = reader.read(&mut input).await.unwrap();
                    let s = std::str::from_utf8(&input[..len]).unwrap();
                    if s.len() == 0 {
                        return;
                    }
                    stdout.write_all(s.as_bytes()).await.unwrap();
                    stdout.flush().await.unwrap();
                };
            },
        };
    }
}
