#![forbid(unsafe_code)]

use omarchy_command_runner::external_power_present;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let configured_path = std::env::var_os("OMARCHY_POWER_SUPPLY_PATH");
    let power_supply_path = configured_path
        .as_deref()
        .filter(|path| !path.is_empty())
        .map(Path::new)
        .unwrap_or_else(|| Path::new("/sys/class/power_supply"));

    if external_power_present(power_supply_path) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
