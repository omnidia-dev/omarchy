use std::ffi::{OsStr, OsString};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

#[cfg(unix)]
mod unix;

#[derive(Debug, Clone)]
pub struct Invocation {
    program: PathBuf,
    arguments: Vec<OsString>,
    environment: Vec<(OsString, OsString)>,
    current_directory: Option<PathBuf>,
    timeout: Duration,
}

impl Invocation {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            arguments: Vec::new(),
            environment: Vec::new(),
            current_directory: None,
            timeout: Duration::from_secs(2),
        }
    }

    pub fn args<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.arguments.extend(arguments.into_iter().map(Into::into));
        self
    }

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.environment.push((key.into(), value.into()));
        self
    }

    pub fn current_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.current_directory = Some(path.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Observation {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
}

pub fn observe(invocation: &Invocation) -> io::Result<Observation> {
    #[cfg(unix)]
    {
        unix::observe(invocation)
    }
    #[cfg(not(unix))]
    {
        let _ = invocation;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "the command corpus currently requires Unix process groups",
        ))
    }
}

pub fn assert_differential_match(existing: &Invocation, candidate: &Invocation) {
    let existing = observe(existing).expect("observe existing command");
    let candidate = observe(candidate).expect("observe candidate command");
    assert!(
        !existing.timed_out && !candidate.timed_out,
        "timeouts cannot qualify command parity"
    );
    assert!(
        existing.exit_code.is_some() && candidate.exit_code.is_some(),
        "signal termination cannot qualify command parity"
    );
    assert_eq!(candidate, existing, "command behavior drift");
}

pub fn os(value: impl AsRef<OsStr>) -> OsString {
    value.as_ref().to_os_string()
}
