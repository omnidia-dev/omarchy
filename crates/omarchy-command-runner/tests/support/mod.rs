use std::ffi::{OsStr, OsString};
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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
    let mut command = Command::new(&invocation.program);
    command
        .args(&invocation.arguments)
        .envs(invocation.environment.iter().cloned())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(current_directory) = &invocation.current_directory {
        command.current_dir(current_directory);
    }

    let mut child = command.spawn()?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let stdout_reader = thread::spawn(move || read_all(stdout));
    let stderr_reader = thread::spawn(move || read_all(stderr));
    let deadline = Instant::now() + invocation.timeout;
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait()? {
            break (status, false);
        }
        if Instant::now() >= deadline {
            child.kill()?;
            break (child.wait()?, true);
        }
        thread::sleep(Duration::from_millis(2));
    };

    Ok(Observation {
        exit_code: status.code(),
        stdout: stdout_reader
            .join()
            .expect("stdout reader thread did not panic")?,
        stderr: stderr_reader
            .join()
            .expect("stderr reader thread did not panic")?,
        timed_out,
    })
}

pub fn assert_differential_match(existing: &Invocation, candidate: &Invocation) {
    let existing = observe(existing).expect("observe existing command");
    let candidate = observe(candidate).expect("observe candidate command");
    assert_eq!(candidate, existing, "command behavior drift");
}

fn read_all(mut pipe: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    pipe.read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub fn os(value: impl AsRef<OsStr>) -> OsString {
    value.as_ref().to_os_string()
}
