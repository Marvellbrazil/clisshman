# Session To-Do List

> **Goal**: Build Clisshman - Rust CLI SSH Manager with CRUD, native OpenSSH, ping, interactive CLI, TUI, and multi-platform distribution.
> **Status**: Active (0/6 Completed)
> **Created**: 2026-03-30

---

## Tasks

- [ ] **#1** Inisialisasi Cargo crate, domain model (Session, Config, Tags sorting A-Z), storage JSON atomik + file lock (`fs2`).
- [ ] **#2** Clap CLI parser & handler CRUD lengkap (`add`, `list`, `show`, `edit`, `delete`, `config`) + format table & `--json`.
- [ ] **#3** Subsystem SSH native connect (exec Unix / spawn Windows + passthrough args) & ping (TCP port default + `--icmp`).
- [ ] **#4** Interactive CLI wizard (`inquire`) & `clisshman doctor`.
- [ ] **#5** Terminal User Interface / TUI (`ratatui` + `crossterm`).
- [ ] **#6** Import/Export SSH config, completions, dan konfigurasi distribusi multi-platform.

---

## Current Focus

- **Active**: #1 Inisialisasi Cargo crate, domain model, storage JSON atomik.
- **Target Outcome**: `cargo check` and `cargo test` pass for core domain and storage.

---

## Completed

---

<!-- AUTOMATIC CLEANUP CONTRACT:
This file is temporary. When all tasks above are checked [x], the agent will automatically delete TODOLIST.md.
-->
