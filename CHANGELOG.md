# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-30

### Added

- **Core & Storage**:
  - Domain models for `Session`, `AppConfig`, and `Registry`.
  - Atomic JSON persistence with `fsync` and process-isolated temporary files.
  - Advisory file locking via `fs2` for concurrent write safety.
  - Tag normalization with comma splitting, whitespace trimming, case-insensitive deduplication, and alphabetical sorting.
  - Hybrid identifier engine supporting incremental integers and UUID v4 with 8-character short display prefixes.
- **CLI Commands (`clap` v4)**:
  - Full CRUD operations: `add`, `list`, `show`, `edit`, `delete`, `config`.
  - Positional target syntax parsing (`user@host:port`).
  - Shorthand aliases `-c` (`--connect`) and `-P` (`--ping`).
  - Multi-criteria filtering on `list` using strict AND matching logic.
  - Visual terminal tables via `comfy-table` with automatic color suppression for `NO_COLOR` and non-TTY pipes.
  - Global `--json` switch for machine-readable output across query commands.
- **Connection & Diagnostics**:
  - Native OpenSSH session runner using process image replacement (`exec`) on Unix and stdio inheritance on Windows.
  - Raw OpenSSH passthrough argument forwarding using `--`.
  - Network reachability checks: TCP connection check to session ports and ICMP ping check via system ping utilities.
  - System diagnostics tool (`clisshman doctor`) for checking OpenSSH availability and storage folder write permissions.
- **Interactive UI**:
  - Interactive terminal wizard (`clisshman cli`) built on `inquire` with arrow-based delete confirmations.
  - Full-screen Terminal User Interface (`clisshman tui`) built on `ratatui` and `crossterm` featuring live search, session inspection, inline TCP ping, and inline SSH session launches.
  - Default binary invocation routing to TUI in interactive terminals and `--help` in piped contexts.
- **Interoperability & Distribution**:
  - OpenSSH configuration export (`clisshman export`) and import (`clisshman import`).
  - Shell completion generation (`clisshman completions`) for `bash`, `zsh`, `fish`, and `powershell`.
  - Multi-platform npm wrapper package with platform-specific `optionalDependencies`.
  - Windows Package Manager (`winget`) portable manifest definition.
  - Automated Release Please GitHub Actions workflow for Rust package lifecycle tracking.
  - Dual MIT and Apache-2.0 open-source licensing.
