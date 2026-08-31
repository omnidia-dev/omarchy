use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

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

fn bash_result(root: &Path, extra_arguments: &[&str]) -> Output {
    Command::new(repository_root().join("bin/omarchy-power-present"))
        .args(extra_arguments)
        .env("OMARCHY_POWER_SUPPLY_PATH", root)
        .output()
        .expect("run Bash power helper")
}

fn rust_result(root: &Path, extra_arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_omarchy-power-present-rs"))
        .args(extra_arguments)
        .env("OMARCHY_POWER_SUPPLY_PATH", root)
        .output()
        .expect("run Rust power helper")
}

fn assert_differential_match(fixture: &Fixture, extra_arguments: &[&str]) {
    let bash = bash_result(&fixture.root, extra_arguments);
    let rust = rust_result(&fixture.root, extra_arguments);

    assert_eq!(rust.status.code(), bash.status.code(), "exit status drift");
    assert_eq!(rust.stdout, bash.stdout, "stdout drift");
    assert_eq!(rust.stderr, bash.stderr, "stderr drift");
}

#[test]
fn matches_connected_mains_and_usb_supplies() {
    for (label, supply_type) in [
        ("mains", b"Mains\n".as_slice()),
        ("usb", b"USB\n".as_slice()),
    ] {
        let fixture = Fixture::new(label);
        fixture.supply("AC0", supply_type, b"1\n");
        assert_differential_match(&fixture, &[]);
        assert!(rust_result(&fixture.root, &[]).status.success());
    }
}

#[test]
fn matches_offline_battery_malformed_and_empty_inventories() {
    let offline = Fixture::new("offline");
    offline.supply("AC0", b"Mains\n", b"0\n");
    assert_differential_match(&offline, &[]);

    let battery = Fixture::new("battery");
    battery.supply("BAT0", b"Battery\n", b"1\n");
    assert_differential_match(&battery, &[]);

    let malformed = Fixture::new("malformed");
    malformed.supply("AC0", b"Mains \n", b"1\n");
    assert_differential_match(&malformed, &[]);

    let empty = Fixture::new("empty");
    assert_differential_match(&empty, &[]);

    let hidden = Fixture::new("hidden");
    hidden.supply(".AC0", b"Mains\n", b"1\n");
    assert_differential_match(&hidden, &[]);
}

#[test]
fn matches_multiple_trailing_newlines_and_ignored_arguments() {
    let fixture = Fixture::new("newlines-and-argv");
    fixture.supply("AC0", b"USB\n\n", b"1\n\n");
    assert_differential_match(&fixture, &["ignored", "argument with spaces"]);
}
