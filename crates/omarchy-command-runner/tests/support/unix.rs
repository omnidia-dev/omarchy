//! Bounded observation for the controlled Unix compatibility corpus.
//!
//! This is not a sandbox: commands that escape their process group require
//! runner-level containment. No reader threads survive a returned observation.
use super::{Invocation, Observation};
use std::io::{self, Read};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_CAPTURE_BYTES: usize = 1024 * 1024;

pub(super) fn observe(invocation: &Invocation) -> io::Result<Observation> {
    let deadline = Instant::now()
        .checked_add(invocation.timeout)
        .filter(|_| !invocation.timeout.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid observation deadline"))?;
    let (mut stdout, stdout_child) = UnixStream::pair()?;
    let (mut stderr, stderr_child) = UnixStream::pair()?;
    stdout.set_nonblocking(true)?;
    stderr.set_nonblocking(true)?;
    let mut command = Command::new(&invocation.program);
    command
        .args(&invocation.arguments)
        .envs(invocation.environment.iter().cloned())
        .stdin(Stdio::null())
        .stdout(Stdio::from(OwnedFd::from(stdout_child)))
        .stderr(Stdio::from(OwnedFd::from(stderr_child)))
        .process_group(0);
    if let Some(directory) = &invocation.current_directory {
        command.current_dir(directory);
    }
    let mut group = ChildGroup {
        child: command.spawn()?,
        cleaned: false,
    };
    // Command retains its configured descriptors until dropped; those extra
    // writer references must not keep capture endpoints open in the parent.
    drop(command);
    let mut stdout_eof = false;
    let mut stderr_eof = false;
    let result = capture(
        &mut group.child,
        &mut stdout,
        &mut stderr,
        &mut stdout_eof,
        &mut stderr_eof,
        deadline,
    );
    let cleanup = group.terminate(!stdout_eof || !stderr_eof);
    match (result, cleanup) {
        (Ok(observation), Ok(())) => Ok(observation),
        (Err(error), Ok(())) => Err(error),
        (_, Err(error)) => Err(error),
    }
}

fn capture(
    child: &mut Child,
    stdout: &mut UnixStream,
    stderr: &mut UnixStream,
    stdout_eof: &mut bool,
    stderr_eof: &mut bool,
    deadline: Instant,
) -> io::Result<Observation> {
    let mut out = Vec::new();
    let mut err = Vec::new();
    loop {
        if !*stdout_eof {
            *stdout_eof = drain(stdout, &mut out)?;
        }
        if !*stderr_eof {
            *stderr_eof = drain(stderr, &mut err)?;
        }
        let status = child.try_wait()?;
        if status.is_some() && *stdout_eof && *stderr_eof {
            return Ok(Observation {
                exit_code: status.and_then(|status| status.code()),
                stdout: out,
                stderr: err,
                timed_out: false,
            });
        }
        if Instant::now() >= deadline {
            return Ok(Observation {
                exit_code: status.and_then(|status| status.code()),
                stdout: out,
                stderr: err,
                timed_out: true,
            });
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn drain(stream: &mut UnixStream, captured: &mut Vec<u8>) -> io::Result<bool> {
    let mut buffer = [0; 4096];
    // Bound work per poll so a continuously writing stream cannot starve the
    // other stream, child observation, or the shared deadline.
    for _ in 0..16 {
        match stream.read(&mut buffer) {
            Ok(0) => return Ok(true),
            Ok(count) => {
                if count > MAX_CAPTURE_BYTES - captured.len() {
                    return Err(io::Error::other("command capture exceeded its byte limit"));
                }
                captured.try_reserve_exact(count).map_err(io::Error::other)?;
                captured.extend_from_slice(&buffer[..count]);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(false),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(false)
}

struct ChildGroup {
    child: Child,
    cleaned: bool,
}

impl ChildGroup {
    fn terminate(&mut self, group_required: bool) -> io::Result<()> {
        let already_exited = self.child.try_wait()?.is_some();
        let signal = Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{}", self.child.id())])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if !already_exited {
            let _ = self.child.kill();
        }
        self.child.wait()?;
        self.cleaned = true;
        match signal {
            Ok(status) if status.success() => Ok(()),
            Ok(_) if already_exited && !group_required => Ok(()),
            Ok(_) => Err(io::Error::other("command process-group cleanup failed")),
            Err(error) => Err(error),
        }
    }
}

impl Drop for ChildGroup {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.terminate(true);
        }
    }
}
