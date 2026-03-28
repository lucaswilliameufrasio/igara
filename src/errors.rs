use thiserror::Error;

#[derive(Debug, Error)]
pub enum IgaraError {
    #[error("host '{0}' was not found")]
    HostNotFound(String),
    #[error("docker context '{0}' already exists")]
    DockerContextExists(String),
    #[error("required command '{0}' is not available in PATH")]
    MissingCommand(String),
    #[error("operation requires TTY")]
    NotATty,
}
