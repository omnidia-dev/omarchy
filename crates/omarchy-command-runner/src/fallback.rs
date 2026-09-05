use std::io;
use std::path::Path;

/// ENOENT may name a missing interpreter, not a missing executable. Reject
/// existing entries and uncertain inspection, including dangling path links.
pub(super) fn executable_is_absent(path: &Path) -> bool {
    for component in path.ancestors() {
        match std::fs::symlink_metadata(component) {
            Ok(metadata) => {
                return component != path
                    && metadata.is_dir()
                    && !metadata.file_type().is_symlink();
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return false,
        }
    }
    false
}

#[cfg(all(test, unix))]
mod tests {
    use crate::{CompatibilityCommand, ExistingCommand};
    use std::fs;
    use std::io;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "omarchy-fallback-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn existing_executable_with_missing_interpreter_never_falls_back() {
        let fixture = Fixture::new();
        let provider = fixture.0.join("provider");
        let marker = fixture.0.join("fallback-ran");
        fs::write(&provider, b"#!/definitely/missing/interpreter\n").unwrap();
        fs::set_permissions(&provider, fs::Permissions::from_mode(0o700)).unwrap();
        let error = CompatibilityCommand::new(ExistingCommand::new("/usr/bin/touch").arg(&marker))
            .with_provider(ExistingCommand::new(&provider))
            .status()
            .expect_err("interpreter failure must not select another command");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(provider.is_file());
        assert!(!marker.exists());
    }

    #[test]
    fn dangling_executable_and_ancestor_links_never_fall_back() {
        let fixture = Fixture::new();
        let link = fixture.0.join("provider");
        let marker = fixture.0.join("fallback-ran");
        symlink(fixture.0.join("absent-target"), &link).unwrap();
        for provider in [link.clone(), link.join("nested-program")] {
            assert!(!super::executable_is_absent(&provider));
            assert!(
                CompatibilityCommand::new(ExistingCommand::new("/usr/bin/touch").arg(&marker))
                    .with_provider(ExistingCommand::new(provider))
                    .status()
                    .is_err()
            );
            assert!(!marker.exists());
        }
    }
}
