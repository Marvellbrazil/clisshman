# clisshman

Cross-platform SSH connection manager for Linux, macOS, and Windows written in Rust. It provides a direct CLI, an interactive inquiry wizard, and a terminal user interface (TUI) to manage remote host profiles, execute native OpenSSH sessions, verify host reachability via TCP or ICMP, and import or export OpenSSH configurations.

## Requirements

- OpenSSH client (`ssh`) available in your system `PATH`. File transfers additionally require native `scp`. Use OpenSSH 9.0+ for SFTP-based transfers.
- A current stable Rust toolchain when building from source. This branch does not declare an MSRV.

## Installation

### From Source

```bash
cargo install --path .
```

Both `clisshman` and `csm` are installed by Cargo. They share the same commands, configuration, and session registry. `csm` is an executable shorthand, not a shell alias. The npm package maps both names to the same launcher when published. Existing releases and installations do not gain the shorthand until updated.

```bash
csm list
```

```bash
csm completions bash --command-name csm
```

### Distribution status

Build from a checkout for the unreleased features documented below. Check [GitHub Releases](https://github.com/Marvellbrazil/clisshman/releases) for the assets available for a specific version; a source archive is not an executable.

This repository contains an npm launcher, platform-package metadata, and a winget manifest. Those files alone do not publish packages to npm or the winget catalog. The local npm development link is not evidence of a public installation. Publishing and additional binary build targets are separate release work.

## Modes of Operation

1. **CLI Mode**: Fast commands for direct shell use and scripts (`clisshman add`, `list`, `connect`, etc.).
2. **Interactive Wizard (`clisshman cli`)**: Step-by-step terminal prompt interface powered by `inquire` for adding, editing, or connecting without memorizing flags.
3. **Terminal UI (`clisshman tui`)**: Full-screen interface powered by `ratatui` and `crossterm` with live search, session inspection, TCP latency checks, and inline SSH execution.
4. **Menu (`clisshman`)**: In an interactive terminal, choose CLI, TUI, or Help. First-run setup selects integer or UUID IDs without resetting an existing configuration. Without a terminal, no-argument invocation prints help and exits nonzero. Explicit commands never open a setup wizard.

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

### Duplicate and Quick connect

Duplicate copies connection fields, description, group, and tags. It creates a new ID and timestamps, resets favorite/history, and leaves the source unchanged. It does not copy private key files or tunnel profiles.

```bash
clisshman duplicate web-prod --name web-staging --host staging.internal --group staging
```

Use `--from-group production` to disambiguate a source name. For `edit`, `--from-group` selects the original group and `--group` sets the destination group. Set and clear flags cannot be combined for the same field.

```bash
clisshman favorite web-prod
```

```bash
clisshman favorite web-prod --unset
```

```bash
clisshman list --favorites
```

```bash
clisshman list --recent --limit 10
```

```bash
clisshman quick prod
```

```bash
clisshman connect
```

Quick connect requires an interactive terminal. Search matches name, group, host, username, and tags with fuzzy matching. For equally strong matches, favorites precede recent attempts, then names and IDs give a stable order. Commands with an explicit target still resolve an exact ID, UUID prefix of at least eight characters, or exact case-insensitive name. Ambiguous names open a picker only in an interactive terminal without `--json`; scripts must specify an unambiguous target.

`last_connect_attempt_at` records an attempt before launching SSH, not a successful login. Dry-runs, ping, and cancelled selections do not change it. Favorites and history stay in the local registry and JSON backups; no telemetry is sent.

### Saved foreground tunnels

Store local (`-L`), remote (`-R`), or dynamic SOCKS (`-D`) forwarding profiles linked to a session ID.

```bash
clisshman tunnel add --name database --session web-prod --kind local --listen-port 15432 --destination-host db.internal --destination-port 5432
```

```bash
clisshman tunnel add --name socks --session web-prod --kind dynamic --listen-port 1080
```

```bash
clisshman tunnel list
```

```bash
clisshman tunnel show database
```

```bash
clisshman tunnel edit database --listen-port 15433
```

```bash
clisshman tunnel run database --dry-run
```

```bash
clisshman tunnel run database
```

```bash
clisshman tunnel delete database
```

Bindings default to `127.0.0.1`. Non-loopback binding requires `--allow-public-bind`; the listener may then be reachable from other machines. Remote forwarding remains subject to SSH server policy, including `GatewayPorts`. Dynamic profiles do not accept destination fields. Local and remote profiles require both destination host and port.

Tunnels use native `ssh -N -o ExitOnForwardFailure=yes` and run in the foreground. Ctrl+C stops the run; there is no background daemon or automatic restart. A running listener does not prove its destination service is healthy. Delete or reassign a tunnel before deleting its referenced session.

### Copy files with native SCP

Upload files to an existing remote directory using a saved session's username, port, identity file, and jump host:

```bash
clisshman scp 3 --file "./report.pdf" --to "/home/deploy/uploads/"
```

```bash
csm scp 3 -f "./report.pdf" -f "./notes.txt" --to "/home/deploy/uploads/"
```

```bash
csm scp 3 -f "./build" --to "/var/www/" --recursive
```

Download flips the source and destination: `--file` is remote and `--to` is an existing local directory.

```bash
csm scp 3 --file "/var/log/app.log" --to "./downloads/" --download
```

```bash
csm scp 3 --file "./report.pdf" --to "/tmp/" --dry-run --json
```

Sources use repeated `-f/--file`, not comma separation. `-g/--group` disambiguates session names. Directories require `-r/--recursive`; native SCP can follow symlinks and copy contents outside the selected tree. Paths with spaces must be quoted in your shell. Remote paths are literal POSIX paths; wildcard patterns and shell syntax are rejected. Windows UNC local paths are not supported by this wrapper.

Every transfer previews its command and warns about overwrite. Confirm with `y/N` (default No); scripts must pass `-y/--yes`. Confirmation authorizes the transfer and possible overwrite, not a no-clobber guarantee. `--dry-run` validates local paths and prints a preview without contacting the server, prompting, changing history, or transferring data. `--json` is preview-only because real transfers inherit native progress and authentication prompts. The wrapper does not create remote directories, precheck remote files, or force the legacy SCP protocol. OpenSSH 9.0+ defaults to SFTP; older native clients may require different pathname handling.

The interactive CLI has a Transfer files (SCP) action. In the TUI, `s` opens the same transfer prompts for the selected session, temporarily leaving full-screen mode. Native SCP handles progress, authentication, and exit status, and the TUI restores itself after completion or cancellation.

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

SSH import accepts concrete `Host` aliases with `HostName`, `User`, `Port`, `IdentityFile`, and `ProxyJump`. Unsupported directives and patterns fail explicitly rather than silently changing their meaning. Supply a `User`; Clisshman never invents `root`. Quoted identity paths are supported. Export generates unique safe aliases when display names cannot be used literally.

Import previews conflicts. Overwriting an existing session requires `y/N` confirmation (default No), or `--yes` in scripts, and preserves its ID, creation time, favorite/history, and tunnel references. A changed registry during preview aborts the operation. Existing export files also require confirmation or `--yes`. Parent directories must already exist. Clisshman does not edit `~/.ssh/config` automatically.

```bash
clisshman import ~/.ssh/config --dry-run
```

```bash
clisshman export --format json -o clisshman-backup.json
```

```bash
clisshman import clisshman-backup.json --format json --dry-run
```

JSON backups include tags, favorites, attempt timestamps, and tunnels. Restoring into another registry remaps incoming IDs to avoid collisions and updates tunnel references. Existing tunnel-name conflicts abort without partial changes. SSH export cannot preserve all Clisshman metadata.

### Shell Completions

Generate command autocompletion scripts:

```bash
clisshman completions bash > ~/.local/share/bash-completion/completions/clisshman
clisshman completions zsh > ~/.zfunc/_clisshman
clisshman completions fish > ~/.config/fish/completions/clisshman.fish
clisshman completions powershell >> $PROFILE
```

### Diagnostic Check

Check OpenSSH availability, the resolved storage path, and readable configuration schemas:

```bash
clisshman doctor
```

## Configuration

Settings are saved in `config.json` inside standard OS directories:

- **Windows**: `%APPDATA%\clisshman\clisshman\config\config.json`
- **Linux**: `$XDG_CONFIG_HOME/clisshman/config.json`, falling back to `~/.config/clisshman/config.json`
- **macOS**: `~/Library/Application Support/com.clisshman.clisshman/config.json`

Run `clisshman doctor` to see the resolved path. `sessions.json` is stored beside `config.json`. The diagnostic command checks availability and readable schemas without creating files; saving operations check write permissions.

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

| Key                      | Action                                                    |
| ------------------------ | --------------------------------------------------------- |
| `Up` / `k`, `Down` / `j` | Move the selection                                        |
| `/`                      | Fuzzy search; `Enter` or `Esc` leaves search mode         |
| `Tab` / `t`              | Switch between sessions and tunnels                       |
| `Enter`                  | Connect to the session, or run the selected tunnel        |
| `a` / `e`                | Add or edit the selected session or tunnel                |
| `d`                      | Duplicate the selected session                            |
| `x` / `Delete`           | Delete, with Cancel selected by default                   |
| `f`                      | Toggle favorite on the selected session                   |
| `F` / `R`                | Show only favorites / only sessions with a recent attempt |
| `s`                      | Open SCP upload/download prompts for the selected session |
| `p`                      | TCP ping in the background without blocking the interface |
| `r`                      | Reload from disk                                          |
| `q` / `Esc` / `Ctrl+C`   | Quit; `Esc` first clears search or returns from tunnels   |

In forms, `Tab`/`Enter` and `Shift+Tab` move between fields, `Ctrl+U` clears a field, `Ctrl+S` saves, and `Esc` cancels. Selection follows the item ID across searches and reloads. SSH and tunnel runs suspend the interface and restore it when the child process exits.

## Storage Design & Concurrency

Registry and config updates hold an exclusive `.lock` across the read, validation, mutation, and atomic replacement. Unique temporary files live in the same directory, and data is synced before replacement. A failed mutation does not save partial changes. Interactive confirmations and network calls happen outside the transaction.

Schema v1 loads without resetting data. The first successful write saves the original as `sessions.v1.backup.json` and persists schema v2. IDs and counters remain intact. Unknown future schemas and corrupt data fail with a recovery message rather than being overwritten. Back up the directory before switching to an older application version; older versions cannot safely edit the new schema.

Session and tunnel IDs each have their own namespace and generator counter. Switching `id_type` affects new profiles only. UUID short display IDs may be ambiguous; use the full ID in that case. Names that resemble IDs are rejected. Tags split on commas, trim whitespace, deduplicate case-insensitively while keeping the first spelling, and sort A-Z case-insensitively.

Native SSH handles authentication, passwords, host keys, and ssh-agent. Clisshman stores connection metadata and identity paths, not passwords or copies of private keys. Keep registry files and backups private because hostnames, usernames, and usage history may be sensitive.

## Development checks

```bash
cargo fmt --check
```

```bash
cargo clippy --all-targets -- -D warnings
```

```bash
cargo test --locked
```

```bash
node --test tests/npm_launcher.test.cjs
```

CI runs these checks on Windows, Linux, and macOS. Unit tests use temporary storage and local listeners rather than real SSH credentials. Terminal interaction, signal handling, and SSH server forwarding policies also need manual testing on the target shell. Git Bash/mintty and WSL can differ from native Windows terminals; use the direct CLI if terminal capabilities are unavailable.

## License

Dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))
