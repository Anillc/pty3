use thiserror::Error;

pub type Result<T> = std::result::Result<T, PtyError>;

#[derive(Debug, Error)]
pub enum PtyError {
    // TODO: use io::Result
    #[error("failed to run syscall")]
    SyscallFailed(std::io::Error),
}