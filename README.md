# Omarchy fork

This repository maintains a fork of [omacom/omarchy](https://github.com/omacom/omarchy) on the existing `quattro` default branch. It retains the Arch Linux setup, command tools, configuration, migrations, and Quickshell desktop implementation. Fork changes and issues belong in this repository; upstream documentation describes the inherited system.

Working OS and setup behavior comes first. The current runtime remains Bash, QML and the existing supporting implementation. The fork also contains an [unwired Rust command-runner crate](crates/omarchy-command-runner/README.md) for incremental compatibility work. Adding that crate did not replace the installed command routes or convert the desktop to Rust.

## Entry points and source map

| Path | Role |
| --- | --- |
| [bin/omarchy](bin/omarchy) | Command router and command-group discovery |
| [bin](bin) | Existing `omarchy-*` commands, including setup, update and provisioning tools |
| [install](install) | System setup and provisioning implementation |
| [crates/omarchy-command-runner](crates/omarchy-command-runner) | Fork's Rust compatibility library and unwired power-detection helper |
| [shell](shell) | Quickshell desktop and plugins |
| [config](config), [default](default), [etc](etc), [applications](applications), [themes](themes) | Configuration, package inputs, desktop entries and themes |
| [migrations](migrations) | Existing per-user migration scripts |
| [test](test) | CLI, shell, acceptance and fork compatibility checks |
| [docs](docs) | System reference, layout and update contracts |
| [manual](manual) | End-user manual, starting with [getting started](manual/02-getting-started.md) and the [CLI](manual/14-omarchy-cli.md) |
| [agents/skills](agents/skills) | Contributor procedures selected by [AGENTS.md](AGENTS.md) |

On an already installed Omarchy system, command discovery starts with `omarchy --help` and `omarchy commands`. Installation and updates modify host state. Consult the [file layout](docs/file-layout.md), [update process](docs/update-process.md), and [manual](manual) before using setup or update commands. Cloning this repository alone does not install an OS or provide the external Arch packages described by those references.

## Fork development

The Rust workspace in [Cargo.toml](Cargo.toml) currently contains `omarchy-command-runner`. It forwards arguments and process status to existing commands and provides a provider selector; it is not wired into installed routing. Its power-detection helper has differential tests against the Bash implementation. See the crate README for those limits.

[Testing](docs/testing.md) describes the CLI and shell suites. [test/public-parity](test/public-parity) defines the fork's exact-upstream comparison, Rust checks and content audit; it expects an upstream Git ref and pins Rust 1.95.0 for its checks. Graphical acceptance tests require a disposable VM. This documentation adjustment does not claim a fresh OS build, installation or parity pass.

Full native Rust conversion and 100% memory-safe Rust using Lustr/BlueFe are future goals. The current Bash/QML runtime and Rust command forwarding are not evidence of that result. Replacements must keep working builds and prove behavior and failure-path compatibility before they take over a route.

## Ownership and compatibility

The [design and divergence record](docs/DIVERGENCE.md) records the verified upstream parent, fork-specific work and retained layout contracts. The upstream-only project pitch and duplicate manual catalog have been replaced here by the fork's current implementation map. The complete manual remains under `manual/`, and the previous README remains in Git history.

Omarchy commands, package paths, migrations, branding resources and the `quattro` branch name remain intact for current consumers. This fork does not claim to be the upstream distribution or to publish upstream packages. The inherited implementation and contributor history remain credited through source history and notices. See [LICENSE](LICENSE) for the unchanged MIT terms.
