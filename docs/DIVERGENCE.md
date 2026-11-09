# Omarchy fork design and divergence record

Maintain this record when the fork changes source layout, routing, package ownership, supported compilers or installed behavior. Pair implementation changes with exact references and meaningful compatibility checks.

## Provenance

GitHub's repository API verified both fork parent and source as [omacom/omarchy](https://github.com/omacom/omarchy). The inspected `quattro` baseline was `9fc7dd1395a3fdf0594701797fc71791d97b0852`. This identifies provenance, not equality with today's upstream head.

The fork owns its changes and primary README. The README now describes the actual source and entry points instead of presenting the upstream author's project as this fork's identity. The full end-user manual remains in `manual/`; source history preserves the former README. Upstream code, licensing, branding assets and credits remain intact.

## Existing fork work

`Cargo.toml` adds a workspace containing `crates/omarchy-command-runner`. Its README and implementation describe an unwired process compatibility library, a provider selector and an unwired Rust power-detection helper. Existing `bin/` scripts still implement installed commands. `test/public-parity` checks Rust formatting, linting and tests, the power helper's differential test and the public-content audit against a specified upstream ref.

`.github/workflows/upstream-quattro-sync.yml` prepares an exact candidate from canonical upstream `quattro`, validates it without write credentials, reconstructs the same Git object in a separate publication job, and opens a fork PR. Its existence does not establish that a schedule is enabled, that checks passed, or that a merge was accepted. This adjustment does not change or resume that workflow.

## Required layout exceptions

| Retained path or name | Contract and reason | Requirement before changing |
| --- | --- | --- |
| `bin/omarchy`, `bin/omarchy-*` | The router discovers commands by executable filename and metadata; scripts and desktop tools call those names. | Audit routes, aliases and all callers, then run focused CLI and compatibility checks. |
| `install/`, `default/`, `config/`, `etc/`, `applications/`, `themes/`, `shell/` | `docs/file-layout.md` maps these trees into external `omarchy` and `omarchy-settings` packages. Runtime code uses `OMARCHY_PATH`; Quickshell and setup code consume these paths. | Coordinate external package manifests and test installation, upgrades, desktop behavior and rollback. |
| `migrations/` and migration filenames | Installed per-user completion markers track migration filenames. Renaming can replay work or lose continuity. | Preserve migration identities and test existing-state upgrades. |
| `manual/`, `docs/`, `agents/skills/` | `AGENTS.md` separates end-user material, system reference and contributor procedures. | Keep audience-specific documentation in its existing tree. |
| `crates/omarchy-command-runner` | Workspace membership and fork parity commands select this crate. It deliberately retains existing commands as the behavior reference. | Prove each replacement route before activation; process forwarding alone is not a Rust conversion. |
| `quattro`, package and branding names | Upstream sync explicitly targets `quattro`; installed commands, configuration and package inputs use Omarchy names. | Audit active consumers and qualify a coordinated migration before renaming. |

No physical source move was made. Existing paths are runtime and external package inputs, not an obsolete vendor dump. A new top-level wrapper directory would break those contracts without delivering a working replacement.

## Future Rust and memory-safety work

Full native Rust conversion and 100% memory-safe Rust using Lustr/BlueFe are future goals. Today the setup and desktop retain Bash, QML and supporting components. The Rust workspace declares edition 2024 and Rust 1.85 as its minimum; the fork parity script selects Rust 1.95.0. Neither establishes current Lustr/BlueFe compatibility or whole-system memory safety.

Future replacements must audit native libraries, unsafe bindings, generated code, child processes and platform dependencies. Preserve behavior, authority, state continuity and recovery, and qualify working builds before replacing installed code. A wrapper that launches a script does not meet the conversion goal.

## Validation boundary

The ownership adjustment changes only the primary README and this record. Bounded link, source-path, whitespace and content checks cover those documents. No host operation, installation, service change, product build or fresh graphical acceptance test is part of this adjustment.
