# Workflows

Use the installed command's help to confirm availability. These examples use placeholder session ID `3`, tunnel ID `2`, and illustrative paths. Replace them with explicitly selected targets and authorized paths. Do not execute every example as a script.

## Discover and select

Inspect matching profiles, minimizing disclosure:

```bash
clisshman list --group production --json
```

Select a full ID from that result and inspect it:

```bash
clisshman show 3 --json
```

Check port reachability only when network access to that host is authorized:

```bash
clisshman ping 3 --timeout 5 --json
```

An unsuccessful ping does not prove SSH authentication is broken. A successful TCP handshake does not prove login will succeed. If no profile matches, ask for connection details rather than guessing a host, username, or key.

## Add or update a profile

After the user supplies and authorizes the connection metadata:

```bash
clisshman add deploy@api.internal:2222 --name api-worker --group backend --identity "~/.ssh/id_ed25519" --json
```

Inspect the returned ID and use it for later operations. If modifying an existing profile, first inspect it and update only the requested fields. `edit --tag` replaces tags rather than appending; do not discard existing tags unintentionally. There is no general dry-run for CRUD commands.

## Preview and connect

```bash
clisshman connect 3 --dry-run --json
```

The preview has a `program` and argument array. Inspect it as data; do not shell-evaluate a rendered command string. Prefer invoking the clisshman command itself.

If the user wants a session and the execution environment can support its authentication and lifecycle:

```bash
clisshman connect 3
```

Real connections inherit native streams and can block for authentication or an interactive shell. They update attempt history before launch. An agent without a suitable terminal should stop after preview and explain the limitation, not open a hanging connection or switch to another client.

For an authorized OpenSSH option, preview its placement first:

```bash
clisshman connect 3 --dry-run --json -- -o ConnectTimeout=5
```

This is OpenSSH argument passthrough, not a remote-execution API. There is no documented `clisshman exec` or JSON command-result interface. If the task specifically requires remote command execution/capture, explain that capability gap and request approval before choosing an alternative. Do not install Paramiko automatically.

## Upload or download

Preview upload to an existing remote directory:

```bash
clisshman scp 3 --file "./report.pdf" --to "/home/deploy/uploads/" --dry-run --json
```

After authorization for transfer and possible overwrite:

```bash
clisshman scp 3 --file "./report.pdf" --to "/home/deploy/uploads/" --yes
```

Preview download into an existing local directory:

```bash
clisshman scp 3 --file "/var/log/app.log" --to "./downloads/" --download --dry-run --json
```

Run the same download without `--dry-run --json`, adding `--yes` only after authorization. Repeat `--file` for multiple sources. Add `--recursive` only for intended directory transfers, taking symlink traversal into account. Native output and exit status are the actual transfer result; a preview does not check remote files or prove success.

## Saved tunnel

Inspect existing definitions first:

```bash
clisshman tunnel list --json
```

If the requested definition does not exist, create it after authorization:

```bash
clisshman tunnel add --name database --session 3 --kind local --listen-port 15432 --destination-host db.internal --destination-port 5432 --json
```

Use the ID returned by creation, not the example ID below:

```bash
clisshman tunnel run 2 --dry-run --json
```

Run only with an agreed lifecycle and a terminal/process tool that can stop the foreground process:

```bash
clisshman tunnel run 2
```

Default binding is loopback. Stop the foreground process to stop this run. Deleting its saved profile is not a process-control operation. Do not background it silently or infer destination health from a running listener.

## Backup and import

Export to a user-approved new path with an existing parent directory:

```bash
clisshman export --format json --output "./clisshman-backup.json"
```

Backups contain host metadata, favorites, attempt history, and tunnels, not application settings or private keys. Keep them private. An existing export requires overwrite authorization before supplying `--yes`.

Preview restoring/importing:

```bash
clisshman import "./clisshman-backup.json" --format json --dry-run --json
```

Review session/tunnel counts and conflicts, then obtain authorization for the import and any overwrites. Importing a backup into the same registry can fail even in dry-run when tunnel names already exist; `--yes` does not override tunnel-name conflicts. Restore into an intended separate registry or resolve names through supported commands after authorization.

```bash
clisshman import "./clisshman-backup.json" --format json --yes --json
```

The import prints its plan before applying it. Check exit status and inspect affected records afterward. IDs of new records are remapped, so discover their new full IDs. A zero exit code alone can also represent a cancelled operation in interactive use.

## Reporting

Report the selected operation, minimal target identity, whether it was previewed or executed, exit status, and any verified state change. Do not label reachability as successful login or an attempt timestamp as a successful connection. Redact sensitive output when the task does not need it.
