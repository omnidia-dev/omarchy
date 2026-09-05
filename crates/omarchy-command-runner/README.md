# Omarchy command runner

This unwired library is the compatibility base for converting selected Omarchy commands to Rust one at a time. It executes an existing command directly, inherits the caller's environment and working directory, forwards arguments without shell parsing, and returns the original process status or output.

It also provides a generic compatibility selector. With no provider configured, the existing command runs unchanged. An optional provider falls back only when its absolute executable path is absent; a provider that starts owns that invocation even when it exits unsuccessfully. Required-provider mode never falls back.

Adding the crate changes no command route. Existing scripts remain the runtime implementation until a separate change replaces one route and proves equivalent behavior.

The workspace also builds an unwired `omarchy-power-present-rs` helper. Differential tests compare its exit status, stdout, and stderr with `bin/omarchy-power-present` across injected power-supply inventories. The installed command remains the Bash implementation.
