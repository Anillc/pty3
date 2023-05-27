use thiserror::Error;

pub type Result<T> = std::result::Result<T, PtyError>;

#[derive(Debug, Error)]
pub enum PtyError {
  #[error("failed to run syscall")]
  SyscallFailed,
}