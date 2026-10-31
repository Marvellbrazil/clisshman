# Command reference

This reference describes the CLI in this checkout. Check the installed binary's `--version` and each command's `--help` before using it. `csm` accepts the same arguments as `clisshman`.

## Common behavior

- Global `--json` selects structured output where supported. It can appear before or after a subcommand. `-h/--help` displays help; `-V/--version` displays version through Clap.
- Session targets resolve an exact ID, a UUID prefix of at least eight characters, or an exact case-insensitive name. Names can require group disambiguation. Prefer full IDs from JSON output.
- Session selection flags `-g/--group` usually disambiguate a name. `edit` and `duplicate` use `--from-group` for source selection because `--group` changes the destination group.
- List filters use case-insensitive substring matching for name, host, and username; case-insensitive exact matching for group and tags; exact port matching; and AND composition. Tags are comma-delimited and normalized. `--recent` restricts results to sessions with an attempt timestamp and sorts newest first. `--limit` must be positive.
- Missing targets or ambiguous names may open interactive selection in a terminal without JSON. Automation should use exact IDs and explicit commands.
- Most application errors exit 1. With `--json`, top-level application errors go to stderr as `{"success":false,"error":{"code":"COMMAND_FAILED","message":"..."}}`. Clap argument errors/help use Clap's own output, not this envelope.
- Success output is command-specific: arrays, objects, scalars, previews, or no stdout. Do not expect a universal `success` field. A cancelled confirmation can return 0 with no mutation.

## Session commands

### `add [TARGET]`

Create a profile. `TARGET` accepts `user@host:port`; explicit flags override supplied target fields. A username and host are required after combining both sources. Port defaults to configured `default_port` (initially 22); name defaults to `user@host`.

| Flag | Meaning |
| --- | --- |
| `-n, --name VALUE` | Display name |
| `-g, --group VALUE` | Optional group |
| `-H, --host VALUE` | Hostname or IP |
| `-u, --username VALUE` | SSH username |
| `-p, --port VALUE` | Nonzero port |
| `-i, --identity VALUE` | Identity file path, not key contents |
| `-J, --jump VALUE` | Jump host specification |
| `-d, --description VALUE` | Description |
| `-t, --tag VALUE` | Comma-delimited tags |

Outputs the created session as JSON with `--json`, otherwise text. Creates an ID and timestamps. Names resembling IDs are rejected.

### `list`

Inspect sessions. Flags: `-n/--name`, `-g/--group`, `-H/--host`, `-u/--username`, `-p/--port`, `-t/--tag`, `--favorites`, `--recent`, `--limit N`. Text output is a table; `--json` outputs a session array, including full IDs.

### `show [TARGET]`

Inspect one session. Accepts all `list` filters. With a target, `--group` disambiguates it and remaining filters still apply. Without a target, filters must yield exactly one match in noninteractive/JSON operation. Outputs one session object with `--json`.

### `edit TARGET`

Update a profile. Accepts all `add` flags except positional `TARGET` is the existing profile, not a new connection address. `--from-group VALUE` selects the original group. `--group` sets its new group; supplied tags replace existing tags.

Clear flags: `--clear-tags`, `--clear-group`, `--clear-identity`, `--clear-jump`, `--clear-description`. Setting and clearing the same field conflict. Unspecified fields remain unchanged. Outputs the updated session.

### `duplicate TARGET --name VALUE`

Copy a profile with a required new name. Flags: `--from-group`, `-n/--name`, `-g/--group`, `-H/--host`, `-u/--username`, `-p/--port`. Source connection metadata, description, group, and tags are copied unless overridden where supported. ID/timestamps are new; favorite and attempt history reset. No private key files or tunnel profiles are copied. Outputs the new session.

### `delete TARGET`

Flags: `-g/--group`, `-y/--yes`. Deletion normally requires confirmation. Noninteractive/JSON deletion needs `--yes`; interactive `confirm_delete=false` can skip the prompt. Referencing tunnels must be deleted or reassigned first. Outputs the deleted session when applied.

### `favorite TARGET`

Flags: `-g/--group`, `--unset`. Marks favorite by default; `--unset` clears it. Outputs the updated session.

### `quick [QUERY]`

Interactive fuzzy selector, then connection. Accepts all `list` filters. Requires an interactive terminal and rejects `--json`. Not an automation entry point.

## Connection and transfer

### `connect [TARGET]`

Flags: `-g/--group`, `--dry-run`, `-- [OPENSSH_ARGS]...`. Root command aliases: `-c`, `--connect`.

- `--dry-run` requires a target and prints the planned SSH command without spawning it or recording history.
- With `--dry-run --json`, output is `{"program":"ssh","args":[...]}`.
- Real execution rejects `--json`, records an attempt, and inherits native authentication/output. On Unix the direct CLI replaces itself with SSH; other platforms wait for it.
- Passthrough arguments are inserted before `user@host`. They are OpenSSH options, not a dedicated remote-command argument. There is no `exec` subcommand.

### `ping TARGET`

Flags: `-g/--group`, `--icmp`, `--timeout SECONDS`. Root aliases: `-P`, `--ping`. TCP handshake against the saved port is the default; timeout defaults to configured `ping_timeout_seconds` (initially 3). `--icmp` uses system ping instead. Timeout must be positive. JSON output includes `target`, `host`, `port` (nullable), `ping_type`, `success`, `latency_ms` (nullable), and `message`; exit 0 means reachable, 1 means failure. Does not authenticate or update attempt history.

### `scp TARGET --file PATH --to DIRECTORY`

| Flag | Meaning |
| --- | --- |
| `-g, --group VALUE` | Session disambiguation |
| `-f, --file PATH` | Required source; repeat for multiple sources |
| `--to DIRECTORY` | Required existing destination directory |
| `--download` | Remote sources to local destination; default is upload |
| `-r, --recursive` | Allow directory sources |
| `-y, --yes` | Authorize transfer and possible overwrite without prompt |
| `--dry-run` | Local validation and preview only |

Uses saved user, port, identity, and jump host. Remote paths must be literal POSIX paths, not globs or shell syntax. Local Windows UNC paths are unsupported. Recursive native SCP can follow symlinks beyond the selected tree.

Always previews. Real transfer prompts with default No unless `--yes`; noninteractive transfer needs `--yes`. Real execution rejects `--json`; `--dry-run --json` emits `program`, `args`, `direction` (`upload` or `download`), and `may_overwrite` (`true`). It does not create remote directories, precheck remote files, guarantee no-clobber, or modify connection history. Native SCP controls authentication/progress/exit status. OpenSSH 9.0+ defaults to SFTP-based transfers.

## Tunnels

Session IDs and tunnel IDs are separate namespaces. Tunnel targets resolve an exact ID, UUID prefix of at least eight characters, or exact case-insensitive name. Tunnel management commands output JSON even without `--json`.

### `tunnel add --name VALUE --session TARGET --kind KIND --listen-port PORT`

| Flag | Meaning |
| --- | --- |
| `-n, --name VALUE` | Required tunnel name |
| `--session TARGET` | Required saved session |
| `-g, --group VALUE` | Disambiguate the referenced session |
| `--kind KIND` | Required: `local`, `remote`, or `dynamic` |
| `--bind ADDRESS` | Listener address; default `127.0.0.1` |
| `--listen-port PORT` | Required nonzero listener port |
| `--destination-host HOST` | Required for local/remote, forbidden for dynamic |
| `--destination-port PORT` | Required for local/remote, forbidden for dynamic |
| `--allow-public-bind` | Explicit permission for non-loopback binding |

Creates a saved definition, not a running process. Validate and obtain user authorization before public binding.

### `tunnel list`

Outputs the saved tunnel array. No command-specific flags.

### `tunnel show TARGET`

Outputs one saved tunnel. No command-specific flags.

### `tunnel edit TARGET`

Optional flags: `-n/--name`, `--session`, `-g/--group`, `--kind`, `--bind`, `--listen-port`, `--destination-host`, `--destination-port`, `--allow-public-bind`. `--group` requires `--session`. Unspecified optional fields remain unchanged; changing kind to dynamic clears destination fields. An unchanged bind preserves its existing public-bind permission; changing the bind resets that permission unless `--allow-public-bind` is supplied. Moving back to loopback does not require public-bind permission.

### `tunnel delete TARGET`

Flag: `-y/--yes`. Requires confirmation or explicit `--yes` in automation. Removes the saved definition; it is not a background-process stop command.

### `tunnel run TARGET`

Flag: `--dry-run`. Preview with `--dry-run --json` returns `program` and `args`. Real execution rejects `--json`, records a session attempt, and runs native `ssh -N -o ExitOnForwardFailure=yes` in the foreground until stopped. No daemon, restart policy, PID registry, or destination-health guarantee. Remote binds remain subject to SSH server policy.

## Configuration and maintenance

### `config [KEY] [VALUE]`

No arguments: current config object. Key only: that key's JSON value. Key and value: update and output config. Always outputs JSON. Keys: `id_type` (`integer` or `uuid`), `default_port` (positive port), `confirm_delete` (`true`/`false` or `1`/`0`), `ping_timeout_seconds` (positive integer). Defaults are detailed in [configuration.md](configuration.md).

### `doctor`

No command-specific flags. Checks native SSH availability/version and readable config/registry schemas without creating files. `--json` fields: `ssh_available`, `ssh_version`, `config_path`, `storage_readable`. Exit 0 requires SSH available and schemas readable. Does not test SCP, authentication, remote access, or storage write permission.

### `export`

Flags: `-o/--output PATH`, `--format ssh|json` (default `ssh`), `-y/--yes`. Without output path, writes export contents to stdout. Existing output files require confirmation or `--yes`; parent directories must already exist. Application storage files are forbidden destinations. Use `--format json` for a registry backup; `--json` is not the format selector and is rejected for SSH-format export. Config settings and private key contents are not included.

### `import FILE`

Flags: `--format ssh|json` (default `ssh`), `--dry-run`, `-y/--yes`. Outputs a JSON preview containing `sessions`, `tunnels`, `conflicts`, and `dry_run`. Without `--dry-run`, a conflict-free import writes immediately; conflicts require confirmation or `--yes`. The preview is not a post-write success receipt.

SSH input supports concrete `Host` aliases with `HostName`, `User`, `Port`, `IdentityFile`, and `ProxyJump`. Unsupported directives/patterns fail explicitly; this is not a full OpenSSH parser. A username must be supplied. JSON import remaps new IDs and tunnel references; session overwrites preserve existing IDs, creation time, favorites, and attempt history. Tunnel-name conflicts abort. Concurrent registry changes during preview abort rather than silently overwrite.

### `completions SHELL`

Shells: `bash`, `elvish`, `fish`, `powershell`, `zsh`. Flag: `--command-name clisshman|csm` (default `clisshman`). Writes a shell completion script to stdout, not JSON; does not install it.

### `cli`, `tui`, and bare invocation

`cli` opens prompt workflows; `tui` opens the full-screen interface. Both require a terminal, reject JSON, and can run onboarding. Bare invocation opens onboarding/menu in a terminal; otherwise prints help and exits 1. Avoid these entry points in agents without interactive terminal support.
