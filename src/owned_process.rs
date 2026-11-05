//! Process ownership for cancellation: kill the owned tree and reap its leader.
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub(crate) struct OwnedChild(Option<Child>);

pub(crate) fn spawn_owned(command: &mut Command) -> std::io::Result<OwnedChild> {
    #[cfg(unix)]
    command.process_group(0);
    command.kill_on_drop(true);
    Ok(OwnedChild(Some(command.spawn()?)))
}

impl OwnedChild {
    pub(crate) fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.0.as_mut().and_then(|child| child.stdin.take())
    }

    pub(crate) fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.0.as_mut().and_then(|child| child.stdout.take())
    }

    pub(crate) async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        match self.0.as_mut() {
            Some(child) => child.wait().await,
            None => Err(std::io::Error::other("owned process no longer available")),
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        let Some(mut child) = self.0.take() else {
            return;
        };
        let Some(id) = child.id() else {
            // Already reaped. Never signal a possibly reused process ID.
            return;
        };
        #[cfg(unix)]
        {
            if let Some(pid) = rustix::process::Pid::from_raw(id as i32) {
                // The unreaped leader retains the process-group identity.
                let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
            }
        }
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                #[cfg(windows)]
                {
                    // Keep the owned process handle open until taskkill has used its ID.
                    let mut terminate = Command::new("taskkill");
                    terminate
                        .args(["/F", "/T", "/PID", &id.to_string()])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .kill_on_drop(true);
                    let _ =
                        tokio::time::timeout(std::time::Duration::from_secs(3), terminate.status())
                            .await;
                }
                let _ = child.start_kill();
                let _ = child.wait().await;
            });
        }
        // Without a runtime, Child's kill_on_drop is the last-resort leader kill.
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn dropping_owned_process_terminates_ordinary_local_descendants() {
        let root = tempfile::tempdir().unwrap();
        let marker = root.path().join("survived");
        let mut command = Command::new("sh");
        command
            .args(["-c", "(sleep 0.4; touch \"$1\") & wait", "fixture"])
            .arg(&marker);
        let child = spawn_owned(&mut command).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        drop(child);
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert!(!marker.exists());
    }
}
