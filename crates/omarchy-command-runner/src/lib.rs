#![forbid(unsafe_code)]
//! Direct execution of an existing Omarchy command during incremental Rust conversions.
//!
//! This crate is intentionally not wired into the command router. It preserves the
//! current command as the fallback until a separate migration qualifies a Rust
//! replacement.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output};

/// Whether an unavailable compatibility provider may use the existing command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderPolicy {
    /// Use the existing command only when no provider is configured or its executable is absent.
    Optional,
    /// Return an error instead of using the existing command.
    Required,
}

/// The command that produced a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandOrigin {
    Existing,
    Provider,
}

/// A command result paired with the command that produced it.
#[derive(Debug)]
pub struct CommandResult<T> {
    origin: CommandOrigin,
    value: T,
}

impl<T> CommandResult<T> {
    /// Return the command that produced the result.
    pub fn origin(&self) -> CommandOrigin {
        self.origin
    }

    /// Return the underlying command result.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Consume the routed result and return its underlying value.
    pub fn into_value(self) -> T {
        self.value
    }
}

/// An existing command and its exact argument vector.
///
/// Execution inherits the caller's environment, current directory, and standard
/// streams. Arguments are passed directly to the process without shell expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingCommand {
    program: PathBuf,
    arguments: Vec<OsString>,
}

impl ExistingCommand {
    /// Construct a command without arguments.
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            arguments: Vec::new(),
        }
    }

    /// Append one argument without parsing or rewriting it.
    pub fn arg(mut self, argument: impl Into<OsString>) -> Self {
        self.arguments.push(argument.into());
        self
    }

    /// Append arguments without parsing or rewriting them.
    pub fn args<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.arguments.extend(arguments.into_iter().map(Into::into));
        self
    }

    /// Return the exact program path.
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// Return the exact argument vector.
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    /// Run the existing command with inherited standard streams.
    pub fn status(&self) -> io::Result<ExitStatus> {
        self.command().status()
    }

    /// Run the existing command and capture its output.
    pub fn output(&self) -> io::Result<Output> {
        self.command().output()
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.arguments);
        command
    }
}

impl From<&OsStr> for ExistingCommand {
    fn from(program: &OsStr) -> Self {
        Self::new(program)
    }
}

/// A behavior-preserving route between an existing command and a compatible provider.
///
/// With no provider configured, the existing command runs unchanged. Provider programs must use
/// absolute paths. An optional provider falls back only when it cannot be started because its
/// executable is absent; once a provider starts, its result is authoritative for that invocation.
/// Required providers never fall back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityCommand {
    existing: ExistingCommand,
    provider: Option<ExistingCommand>,
    provider_policy: ProviderPolicy,
}

impl CompatibilityCommand {
    /// Construct a route that preserves the existing command when no provider is configured.
    pub fn new(existing: ExistingCommand) -> Self {
        Self {
            existing,
            provider: None,
            provider_policy: ProviderPolicy::Optional,
        }
    }

    /// Configure a provider command.
    pub fn with_provider(mut self, provider: ExistingCommand) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Set whether the existing command may run when the provider is unavailable.
    pub fn provider_policy(mut self, provider_policy: ProviderPolicy) -> Self {
        self.provider_policy = provider_policy;
        self
    }

    /// Return the existing command.
    pub fn existing(&self) -> &ExistingCommand {
        &self.existing
    }

    /// Return the configured provider, if any.
    pub fn provider(&self) -> Option<&ExistingCommand> {
        self.provider.as_ref()
    }

    /// Run the selected command with inherited standard streams.
    pub fn status(&self) -> io::Result<CommandResult<ExitStatus>> {
        self.run_with(ExistingCommand::status)
    }

    /// Run the selected command and capture its output.
    pub fn output(&self) -> io::Result<CommandResult<Output>> {
        self.run_with(ExistingCommand::output)
    }

    fn run_with<T>(
        &self,
        mut run: impl FnMut(&ExistingCommand) -> io::Result<T>,
    ) -> io::Result<CommandResult<T>> {
        let Some(provider) = &self.provider else {
            if self.provider_policy == ProviderPolicy::Required {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "required compatibility provider is not configured",
                ));
            }

            return run(&self.existing).map(|value| CommandResult {
                origin: CommandOrigin::Existing,
                value,
            });
        };

        if !provider.program().is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "compatibility provider program must be an absolute path",
            ));
        }

        match run(provider) {
            Ok(value) => Ok(CommandResult {
                origin: CommandOrigin::Provider,
                value,
            }),
            Err(error)
                if error.kind() == io::ErrorKind::NotFound
                    && self.provider_policy == ProviderPolicy::Optional =>
            {
                run(&self.existing).map(|value| CommandResult {
                    origin: CommandOrigin::Existing,
                    value,
                })
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "omarchy-command-runner-{label}-{}-{}",
            std::process::id(),
            TEST_PATH_COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn passes_arguments_without_shell_expansion() {
        let output = ExistingCommand::new("/usr/bin/printf")
            .args(["%s\n", "$(printf injected)", "argument with spaces"])
            .output()
            .expect("run printf");

        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).expect("utf-8 output"),
            "$(printf injected)\nargument with spaces\n"
        );
    }

    #[test]
    fn returns_the_existing_commands_exit_status() {
        let status = ExistingCommand::new("/bin/bash")
            .args(["-c", "exit 23"])
            .status()
            .expect("run bash");

        assert_eq!(status.code(), Some(23));
    }

    #[test]
    fn exposes_the_exact_program_and_arguments() {
        let command = ExistingCommand::new("omarchy-menu")
            .arg("toggle")
            .arg("root menu");

        assert_eq!(command.program(), Path::new("omarchy-menu"));
        assert_eq!(
            command.arguments(),
            [OsString::from("toggle"), OsString::from("root menu")]
        );
    }

    #[test]
    fn unchanged_route_uses_the_existing_command() {
        let result = CompatibilityCommand::new(
            ExistingCommand::new("/usr/bin/printf").args(["%s", "existing"]),
        )
        .output()
        .expect("run existing command");

        assert_eq!(result.origin(), CommandOrigin::Existing);
        assert_eq!(result.value().stdout, b"existing");
    }

    #[test]
    fn configured_provider_receives_the_invocation() {
        let result = CompatibilityCommand::new(
            ExistingCommand::new("/usr/bin/printf").args(["%s", "existing"]),
        )
        .with_provider(ExistingCommand::new("/usr/bin/printf").args(["%s", "provider"]))
        .output()
        .expect("run provider command");

        assert_eq!(result.origin(), CommandOrigin::Provider);
        assert_eq!(result.value().stdout, b"provider");
    }

    #[test]
    fn optional_absent_provider_uses_the_existing_command() {
        let result = CompatibilityCommand::new(
            ExistingCommand::new("/usr/bin/printf").args(["%s", "existing"]),
        )
        .with_provider(ExistingCommand::new("/definitely/missing/omarchy-provider"))
        .output()
        .expect("fall back to existing command");

        assert_eq!(result.origin(), CommandOrigin::Existing);
        assert_eq!(result.value().stdout, b"existing");
    }

    #[test]
    fn required_absent_provider_does_not_run_the_existing_command() {
        let marker = test_path("required-provider");
        let error = CompatibilityCommand::new(ExistingCommand::new("/usr/bin/touch").arg(&marker))
            .with_provider(ExistingCommand::new("/definitely/missing/omarchy-provider"))
            .provider_policy(ProviderPolicy::Required)
            .status()
            .expect_err("required provider must fail closed");

        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(!marker.exists());
    }

    #[test]
    fn required_unconfigured_provider_does_not_run_the_existing_command() {
        let marker = test_path("unconfigured-provider");
        let error = CompatibilityCommand::new(ExistingCommand::new("/usr/bin/touch").arg(&marker))
            .provider_policy(ProviderPolicy::Required)
            .status()
            .expect_err("unconfigured required provider must fail closed");

        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(!marker.exists());
    }

    #[test]
    fn provider_failure_is_not_redispatched_to_the_existing_command() {
        let marker = test_path("provider-failure");
        let result = CompatibilityCommand::new(ExistingCommand::new("/usr/bin/touch").arg(&marker))
            .with_provider(ExistingCommand::new("/usr/bin/false"))
            .status()
            .expect("provider started");

        assert_eq!(result.origin(), CommandOrigin::Provider);
        assert!(!result.value().success());
        assert!(!marker.exists());
    }

    #[test]
    fn relative_provider_is_rejected_without_running_the_existing_command() {
        let marker = test_path("relative-provider");
        let error = CompatibilityCommand::new(ExistingCommand::new("/usr/bin/touch").arg(&marker))
            .with_provider(ExistingCommand::new("provider-on-path"))
            .status()
            .expect_err("relative provider must be rejected");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(!marker.exists());
    }
}
