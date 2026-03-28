use thiserror::Error;

#[derive(Debug, Error)]
pub enum IgaraError {
    #[error("host '{0}' was not found")]
    HostNotFound(String),
    #[allow(dead_code)]
    #[error("docker context '{0}' already exists")]
    DockerContextExists(String),
    #[allow(dead_code)]
    #[error("required command '{0}' is not available in PATH")]
    MissingCommand(String),
    #[error("invalid connection mode: {0}")]
    InvalidConnectionMode(String),
    #[error("invalid platform family: {0}")]
    InvalidPlatformFamily(String),
    #[allow(dead_code)]
    #[error("operation requires TTY")]
    NotATty,
}
