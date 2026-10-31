# Configuration and OpenSSH boundaries

## Storage

`config.json` and `sessions.json` live beside each other in the resolved OS configuration directory:

| OS | Default config path |
| --- | --- |
| Windows | `%APPDATA%\clisshman\clisshman\config\config.json` |
| Linux | `$XDG_CONFIG_HOME/clisshman/config.json`, otherwise `~/.config/clisshman/config.json` |
| macOS | `~/Library/Application Support/com.clisshman.clisshman/config.json` |

`CLISSHMAN_CONFIG_DIR` overrides the base directory. An empty override is rejected. Changing it selects another registry; it does not migrate the existing one. Do not change it merely to hide an error or avoid confirmation. Use `doctor --json` to obtain the resolved path without creating files.

Writes use an exclusive `.lock`, validated read-modify-write transactions, and atomic replacement. Registry schema v1 is accepted and backed up as `sessions.v1.backup.json` on the first successful migration write to v2. Unknown future versions and corrupt input are rejected rather than reset. Do not edit storage files directly or downgrade an application against a newer schema without a backup and compatibility review.

## Settings

| Key | Initial default | Accepted values | Effect |
| --- | --- | --- | --- |
| `id_type` | `integer` | `integer`, `uuid` | New IDs only; existing IDs stay unchanged |
| `default_port` | `22` | Integer 1–65535 | Default for new profiles without a port |
| `confirm_delete` | `true` | `true`, `false`, `1`, `0` | Controls interactive session deletion; does not authorize agent deletion |
| `ping_timeout_seconds` | `3` | Positive integer | Default service ping timeout |

`config` outputs JSON even without `--json`. Read all settings:

```bash
clisshman config
```

Read one setting:

```bash
clisshman config default_port
```

After authorization, change only the requested setting:

```bash
clisshman config ping_timeout_seconds 5
```

Session and tunnel IDs have separate counters/namespaces. Keep IDs as strings. JSON registry exports do not include `config.json`; importing a backup is not a restore of application settings.

## OpenSSH owns authentication

Clisshman stores host, username, port, identity path, optional jump host, organizational metadata, favorites, and attempt timestamps. It does not store passwords or copy private keys. Native `ssh` and `scp` handle credentials, ssh-agent, host-key verification, authentication prompts, and SSH server policy.

Keep advanced OpenSSH behavior in the user's existing OpenSSH configuration unless a supported clisshman field or explicit passthrough option is required. Clisshman passes its saved port, identity, jump host, and destination to native tools; native OpenSSH precedence rules still apply. Avoid creating contradictory settings and do not claim that clisshman fully resolves OpenSSH configuration.

SSH import/export only handles a supported subset of directives. It is not a bidirectional editor for arbitrary `~/.ssh/config`, `Include`, or `Match` blocks. Do not import unsupported configuration by silently dropping directives. Do not automatically rewrite the user's OpenSSH config or disable host-key verification.

## Privacy

Registry files, JSON outputs, backups, and previews can disclose internal addresses, usernames, paths, and connection history even though they contain no private key contents. Use filtered CLI inspection instead of reading or publishing the entire registry. Do not upload this data to external services for troubleshooting without explicit permission.
