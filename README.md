# clisshman

Cross-platform SSH connection manager for Linux, macOS, and Windows written in Rust. It provides a direct CLI, an interactive inquiry wizard, and a terminal user interface (TUI) to manage remote host profiles, execute native OpenSSH sessions, verify host reachability via TCP or ICMP, and import or export OpenSSH configurations.

## Requirements

- OpenSSH client (`ssh`) available in your system `PATH`.
- Rust 1.74 or later (if building from source).

## Installation

### From Source

```bash
cargo install --path .
```

### Prebuilt Binaries

Binaries for Linux (`x86_64-unknown-linux-musl`), macOS (`x86_64-apple-darwin`, `aarch64-apple-darwin`), and Windows (`x86_64-pc-windows-msvc`) are attached to each GitHub release.

### npm / npx Runner

```bash
npm install -g clisshman
# or run directly without global installation:
npx clisshman
```

### Windows Package Manager (winget)

```powershell
winget install clisshman
```

## Modes of Operation

1. **CLI Mode**: Fast commands for direct shell use and scripts (`clisshman add`, `list`, `connect`, etc.).
2. **Interactive Wizard (`clisshman cli`)**: Step-by-step terminal prompt interface powered by `inquire` for adding, editing, or connecting without memorizing flags.
3. **Terminal UI (`clisshman tui`)**: Full-screen interface powered by `ratatui` and `crossterm` with live search, session inspection, TCP latency checks, and inline SSH execution.
4. **Auto TTY Launch (`clisshman`)**: Invoking `clisshman` without arguments launches the TUI if executed inside an interactive terminal, or outputs `--help` if stdout is piped or redirected.

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
# Formatted ANSI table
clisshman list

# Multi-criteria filter (AND logic, case-insensitive substring match)
clisshman list -g production -t "web,debian" -u ubuntu

# Structured JSON output for shell automation
clisshman list --json
```

### Connecting to a Session

```bash
# Connect by numeric ID or session name
clisshman connect 1
clisshman -c web-prod

# Dry-run: print the command string without spawning ssh
clisshman connect web-prod --dry-run

# Pass arbitrary OpenSSH arguments using the double-dash separator
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
# Show full session details
clisshman show web-prod

# Edit specific attributes
clisshman edit web-prod -p 2202 -t "staging,web"
clisshman edit web-prod --clear-tags

# Delete session with confirmation prompt
clisshman delete web-prod
clisshman delete web-prod -y # Skip confirmation prompt
```

### OpenSSH Config Import and Export

```bash
# Export all sessions to standard OpenSSH config syntax
clisshman export -o ~/.ssh/config.d/clisshman.conf

# Import existing hosts from an OpenSSH config file
clisshman import ~/.ssh/config
```

### Shell Completions

Generate command autocompletion scripts:

```bash
clisshman completions bash > ~/.local/share/bash-completion/completions/clisshman
clisshman completions zsh > ~/.zfunc/_clisshman
clisshman completions fish > ~/.config/fish/completions/clisshman.fish
clisshman completions powershell >> $PROFILE
```

### Diagnostic Check

Verify OpenSSH binary accessibility and configuration directory permissions:

```bash
clisshman doctor
```

## Configuration

Settings are saved in `config.json` inside standard OS directories:

- **Windows**: `%APPDATA%\clisshman\config.json`
- **Linux**: `~/.config/clisshman/config.json`
- **macOS**: `~/Library/Application Support/clisshman/config.json`

Override the directory by exporting the `CLISSHMAN_CONFIG_DIR` environment variable.

Inspect and update configurations:

```bash
# View all settings
clisshman config

# Toggle ID generation mode between integer and UUID v4
clisshman config id_type uuid
clisshman config id_type integer

# Update defaults
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

- **Atomic File Writes**: Serialized JSON data is written to a process-tagged temporary file (`.tmp.<pid>`), flushed to storage media with `fsync`, and atomically renamed over the target file.
- **File Locking**: Mutating operations acquire an exclusive advisory file lock (`fs2`) on `.lock` before modifying registry files.
- **Hybrid ID Resolver**: When switching between integer and UUID formats, existing entries preserve their identifiers. The resolver routes numeric inputs to integer IDs and alphanumeric values to UUID short prefixes or unique session names.

## License

Dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))
