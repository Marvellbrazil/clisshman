# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-10-31

### Added

* AI Agent Integration:
  * Portable `skills/clisshman/SKILL.md` following the Agent Skills specification.
  * Activation guidance for agents handling saved SSH profiles, connection previews, reachability checks, SCP transfers, and foreground tunnels.
  * Tool-selection instructions that prioritize clisshman for supported operations instead of silently switching to Paramiko or another SSH client.
  * Explicit capability boundaries requiring agents to report unsupported operations rather than invent commands or automatically install alternative clients.
* Agent Reference Documentation:
  * Complete command reference covering 19 top-level commands, six tunnel subcommands, arguments, flags, defaults, and output behavior.
  * Workflow examples for session discovery, connection previews, file transfers, tunnel management, and JSON backup/import.
  * Configuration reference covering storage locations, environment overrides, application settings, and native OpenSSH responsibilities.
  * Troubleshooting guidance for missing binaries, ambiguous targets, authentication prompts, import conflicts, and noninteractive execution.
* Multi-Agent Setup:
  * Installation and activation guidance for Claude Code, Codex, Cursor, Antigravity, OpenCode, GitHub Copilot, Gemini CLI, and Windsurf.
  * Project-scoped and user-wide skill installation paths.
  * Optional installation through the third-party Skills CLI.
  * Progressive disclosure through a concise skill entry point and supporting `references/` documentation.
* Agent Safety Guidelines:
  * Explicit target selection using full session IDs.
  * Dry-run previews before supported connection, transfer, tunnel, and import operations.
  * Authorization requirements for destructive changes, overwrites, and public tunnel bindings.
  * Credential privacy, host-key verification, and safe handling of remote output.
* Native Transfers & Execution:
  * Native SCP upload/download with repeated file sources, recursive directory transfers, preview, overwrite confirmation, and interactive CLI/TUI access.
  * `csm` executable shorthand sharing the same commands and storage, npm command mapping, and shorthand shell completions.
* Session Management & Tunnels:
  * Duplicate session with a new ID and timestamps; favorite/history resets without changing the source.
  * Favorites, recent connection attempts, fuzzy Quick connect, and an interactive picker for targetless connect.
  * Saved foreground local, remote, and dynamic SOCKS tunnel profiles with explicit non-loopback binding consent and session reference protection.
* Storage & Interoperability:
  * Registry schema v2 with backward-compatible metadata defaults and an original-file backup before migration.
  * Full JSON registry backup/import, dry-run import preview, and no-clobber export confirmation.
  * Expanded interactive session/tunnel management and cross-platform CI checks.

### Changed

* Documentation:
  * Expanded the README with agent installation, activation, and verification instructions.
  * Clarified that skill installation does not install the clisshman binary or configure every agent automatically.
  * Documented current limitations, including the absence of an MCP server and a dedicated remote-command execution API.
  * Distinguished Agent Skills format compatibility from behavior verified directly in each supported agent.
* Architecture:
  * Split large source files into focused modules while preserving public imports, command behavior, and storage schemas.

### Fixed

* Core & Persistence:
  * Storage transactions now lock the entire read-modify-write operation instead of only the save.
  * Tags trim and deduplicate case-insensitively, then sort A-Z without changing the first spelling.
  * Shared validation checks final group/name uniqueness, nonzero ports and timeouts, safe field values, and IPv6 target syntax.
  * SSH execution separates direct CLI and returning UI lifecycles; failures do not exit the application service.
  * Import rejects unsupported SSH configuration instead of silently inventing defaults; confirmed overwrite retains existing identity metadata.
  * Noninteractive deletion requires explicit --yes, and JSON output avoids interactive prompts.
  * No-argument interactive startup offers CLI, TUI, and Help; read-only commands avoid creating storage.
  * npm launcher failures produce nonzero status instead of reporting success.
  * Documentation distinguishes local builds and package templates from published installation channels.

## [0.1.0] - 2026-03-30

### Added

* **Core & Storage**:
  * Domain models for `Session`, `AppConfig`, and `Registry`.
  * Atomic JSON persistence with `fsync` and process-isolated temporary files.
  * Advisory file locking via `fs2` for concurrent write safety.
  * Tag normalization with comma splitting, whitespace trimming, case-insensitive deduplication, and alphabetical sorting.
  * Hybrid identifier engine supporting incremental integers and UUID v4 with 8-character short display prefixes.
* **CLI Commands (`clap` v4)**:
  * Full CRUD operations: `add`, `list`, `show`, `edit`, `delete`, `config`.
  * Positional target syntax parsing (`user@host:port`).
  * Shorthand aliases `-c` (`--connect`) and `-P` (`--ping`).
  * Multi-criteria filtering on `list` using strict AND matching logic.
  * Visual terminal tables via `comfy-table` with automatic color suppression for `NO_COLOR` and non-TTY pipes.
  * Global `--json` switch for machine-readable output across query commands.
* **Connection & Diagnostics**:
  * Native OpenSSH session runner using process image replacement (`exec`) on Unix and stdio inheritance on Windows.
  * Raw OpenSSH passthrough argument forwarding using `--`.
  * Network reachability checks: TCP connection check to session ports and ICMP ping check via system ping utilities.
  * System diagnostics tool (`clisshman doctor`) for checking OpenSSH availability and storage folder write permissions.
* **Interactive UI**:
  * Interactive terminal wizard (`clisshman cli`) built on `inquire` with arrow-based delete confirmations.
  * Full-screen Terminal User Interface (`clisshman tui`) built on `ratatui` and `crossterm` featuring live search, session inspection, inline TCP ping, and inline SSH session launches.
  * Default binary invocation routing to TUI in interactive terminals and `--help` in piped contexts.
* **Interoperability & Distribution**:
  * OpenSSH configuration export (`clisshman export`) and import (`clisshman import`).
  * Shell completion generation (`clisshman completions`) for `bash`, `zsh`, `fish`, and `powershell`.
  * Multi-platform npm wrapper package with platform-specific `optionalDependencies`.
  * Windows Package Manager (`winget`) portable manifest definition.
  * Automated Release Please GitHub Actions workflow for Rust package lifecycle tracking.
  * Dual MIT and Apache-2.0 open-source licensing.
