# clisshman

Terminal SSH connection manager for Linux, macOS, and Windows. It provides a CLI, an interactive inquiry wizard, and a full TUI to manage remote host profiles, run native OpenSSH sessions, check reachability, and import or export OpenSSH configurations.

## Requirements

- OpenSSH client (`ssh`) installed and available in your `PATH`.
- Rust 1.74 or later (if building from source).

## Installation

### From Source

```bash
cargo install --path .
```

### Binary Download

Compiled binaries for Linux (`x86_64-unknown-linux-musl`), macOS (`x86_64-apple-darwin`, `aarch64-apple-darwin`), and Windows (`x86_64-pc-windows-msvc`) are published on GitHub Releases.

### npm / npx Runner

```bash
npm install -g clisshman
# or run directly:
npx clisshman
```

### Windows Package Manager (winget)

```powershell
winget install clisshman
```

## Modes of Operation

1. **CLI Mode**: Direct commands for scripting and quick operations (`clisshman add`, `list`, `connect`).
2. **Interactive CLI Wizard (`clisshman cli`)**: Step-by-step prompts powered by `inquire` for adding, editing, and managing connections without remembering flags.
3. **Terminal UI (`clisshman tui`)**: Full-screen interface powered by `ratatui` and `crossterm` with live search, session inspection, TCP latency checks, and inline SSH execution.
4. **Default Invocation (`clisshman`)**: Automatically opens the TUI if executed inside an interactive terminal (TTY), or prints help if stdout is piped.

## CLI Usage

### Adding a Session

```bash
# Explicit flags
clisshman add -H 192.168.1.10 -u ubuntu -p 22 -n web-prod -g production -t "web,debian"

# Positional target syntax
clisshman add deploy@api.internal:2222 -n api-worker -g backend -i ~/.ssh/id_ed25519

# With jump host and description
clisshman add -H db.private -u postgres -J bastion.public -d "Production database"
```

### Listing Sessions

```bash
# Render ASCII/Unicode table
clisshman list

# Multi-criteria filter (AND logic, case-insensitive substring match)
clisshman list -g production -t "web,debian" -u ubuntu

# JSON output for scripting
clisshman list --json
```

### Connecting to a Session

```bash
# Connect by numeric ID or name
clisshman connect 1
clisshman -c web-prod

# Dry-run: print the generated SSH command without running it
clisshman connect web-prod --dry-run

# Forward raw OpenSSH arguments using the double-dash separator
clisshman connect web-prod -- -L 8080:localhost:80 -X
```

### Testing Connection (Ping)

```bash
# TCP handshake check against host and configured port (default)
clisshman ping 1
clisshman -P web-prod

# ICMP ping check
clisshman ping web-prod --icmp

# Custom timeout in seconds
clisshman ping web-prod --timeout 5 --json
```

### Inspecting, Editing, and Deleting

```bash
# Show detailed view
clisshman show web-prod

# Edit fields (replaces tags if provided, or clear them completely)
clisshman edit web-prod -p 2202 -t "staging,web"
clisshman edit web-prod --clear-tags

# Delete session
clisshman delete web-prod
clisshman delete web-prod -y # Non-interactive skip confirmation
```

### OpenSSH Config Import and Export

```bash
# Export all clisshman sessions to standard ~/.ssh/config format
clisshman export -o ~/.ssh/config.d/clisshman.conf

# Import existing hosts from an OpenSSH config file
clisshman import ~/.ssh/config
```

### Shell Completions

Generate auto-completion scripts for your shell:

```bash
clisshman completions bash > ~/.local/share/bash-completion/completions/clisshman
clisshman completions zsh > ~/.zfunc/_clisshman
clisshman completions fish > ~/.config/fish/completions/clisshman.fish
clisshman completions powershell >> $PROFILE
```

### Diagnostic Check

Verify that OpenSSH and file storage directories are accessible:

```bash
clisshman doctor
```

## Configuration

Stored in `config.json` inside standard OS config directories (`%APPDATA%\clisshman\config.json` on Windows, `~/.config/clisshman/config.json` on Linux, `~/Library/Application Support/clisshman/config.json` on macOS). Override the path via the `CLISSHMAN_CONFIG_DIR` environment variable.

Inspect and change settings via CLI:

```bash
# View current configuration
clisshman config

# Toggle between incremental integers (1, 2, 3) and UUID v4
clisshman config id_type uuid
clisshman config id_type integer

# Change defaults
clisshman config default_port 2222
clisshman config confirm_delete 0
clisshman config ping_timeout_seconds 5
```

## TUI Keybindings

| Key                             | Action                                |
| ------------------------------- | ------------------------------------- |
| `Up` / `k`                      | Move cursor up                        |
| `Down` / `j`                    | Move cursor down                      |
| `/`                             | Enter search filter mode              |
| `Enter` / `Esc` (during search) | Exit search mode and focus list       |
| `Enter` (on list)               | Connect to selected SSH session       |
| `p`                             | Run TCP ping test on selected session |
| `r`                             | Reload sessions from disk             |
| `q` / `Esc`                     | Quit TUI                              |

## Storage Design & Concurrency

- **Atomic File Writes**: Write operations write to temporary files first (`.tmp.<pid>`) and call `fsync` before renaming over the target JSON file.
- **File Locking**: Write operations acquire an exclusive advisory lock via `fs2` on `.lock` before modifying `sessions.json`.
- **Hybrid ID Resolver**: When `id_type` changes, existing sessions retain their assigned IDs. The resolver routes pure integer strings to integer IDs and alphanumeric strings to short UUIDs or names.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))
