mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use support::{Invocation, assert_differential_match, observe, os};

static TEST_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "omarchy-power-present-{label}-{}-{}",
            std::process::id(),
            TEST_PATH_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create fixture root");
        Self { root }
    }

    fn supply(&self, name: &str, supply_type: &[u8], online: &[u8]) {
        let supply = self.root.join(name);
        fs::create_dir(&supply).expect("create supply");
        fs::write(supply.join("type"), supply_type).expect("write supply type");
        fs::write(supply.join("online"), online).expect("write online state");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate is nested under repository crates")
        .to_path_buf()
}

fn bash_invocation(root: &Path, extra_arguments: &[&str]) -> Invocation {
    Invocation::new(repository_root().join("bin/omarchy-power-present"))
        .args(extra_arguments.iter().map(os))
        .env("OMARCHY_POWER_SUPPLY_PATH", root)
        .current_dir(root)
}

fn rust_invocation(root: &Path, extra_arguments: &[&str]) -> Invocation {
    Invocation::new(env!("CARGO_BIN_EXE_omarchy-power-present-rs"))
        .args(extra_arguments.iter().map(os))
        .env("OMARCHY_POWER_SUPPLY_PATH", root)
        .current_dir(root)
}

fn assert_power_match(fixture: &Fixture, extra_arguments: &[&str]) {
    assert_differential_match(
        &bash_invocation(&fixture.root, extra_arguments),
        &rust_invocation(&fixture.root, extra_arguments),
    );
}

#[test]
fn harness_compares_argv_environment_working_directory_and_standard_streams() {
    let directory = std::env::temp_dir();
    let script = "printf '%s|%s|' \"$1\" \"$CORPUS_VALUE\"; pwd; printf 'stderr' >&2; exit 17";
    let invocation = || {
        Invocation::new("/bin/bash")
            .args(["-c", script, "bash", "argument with spaces"])
            .env("CORPUS_VALUE", "environment value")
            .current_dir(&directory)
    };

    assert_differential_match(&invocation(), &invocation());
    let observed = observe(&invocation()).unwrap();
    assert_eq!(observed.exit_code, Some(17));
    assert!(
        observed
            .stdout
            .starts_with(b"argument with spaces|environment value|")
    );
    assert_eq!(observed.stderr, b"stderr");
    assert!(!observed.timed_out);
}

#[test]
fn harness_bounds_nonterminating_commands_and_reaps_them() {
    let invocation = Invocation::new("/bin/bash")
        .args(["-c", "while :; do :; done"])
        .timeout(Duration::from_millis(50));
    let started = Instant::now();

    let observed = observe(&invocation).unwrap();

    assert!(observed.timed_out);
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn matches_connected_mains_and_usb_supplies() {
    for (label, supply_type) in [
        ("mains", b"Mains\n".as_slice()),
        ("usb", b"USB\n".as_slice()),
    ] {
        let fixture = Fixture::new(label);
        fixture.supply("AC0", supply_type, b"1\n");
        assert_power_match(&fixture, &[]);
        assert_eq!(
            observe(&rust_invocation(&fixture.root, &[]))
                .unwrap()
                .exit_code,
            Some(0)
        );
    }
}

#[test]
fn matches_offline_battery_malformed_and_empty_inventories() {
    let offline = Fixture::new("offline");
    offline.supply("AC0", b"Mains\n", b"0\n");
    assert_power_match(&offline, &[]);

    let battery = Fixture::new("battery");
    battery.supply("BAT0", b"Battery\n", b"1\n");
    assert_power_match(&battery, &[]);

    let malformed = Fixture::new("malformed");
    malformed.supply("AC0", b"Mains \n", b"1\n");
    assert_power_match(&malformed, &[]);

    let empty = Fixture::new("empty");
    assert_power_match(&empty, &[]);

    let hidden = Fixture::new("hidden");
    hidden.supply(".AC0", b"Mains\n", b"1\n");
    assert_power_match(&hidden, &[]);
}

#[test]
fn matches_multiple_trailing_newlines_and_ignored_arguments() {
    let fixture = Fixture::new("newlines-and-argv");
    fixture.supply("AC0", b"USB\n\n", b"1\n\n");
    assert_power_match(&fixture, &["ignored", "argument with spaces"]);
}
