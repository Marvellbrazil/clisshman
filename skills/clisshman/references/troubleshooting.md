# Troubleshooting

| Symptom | Meaning and next step |
| --- | --- |
| `clisshman` not found | Try `csm --version`. If both are missing, report that the binary must be installed; a skill does not install it. Ask before installation or PATH changes. |
| Documented flag rejected | Installed version may differ from this checkout. Inspect `--version` and the command's `--help`; do not invent compatibility flags. |
| Agent ignores this skill | Verify that the host supports Agent Skills, the complete `clisshman` folder is in its skill discovery directory, and the session has refreshed its skill list. Explicitly request the clisshman skill. Copying only `SKILL.md` breaks its reference links. |
| Agent chooses Paramiko | Invoke this skill explicitly and require supported operations through clisshman. If the request needs unsupported remote execution, report the gap and seek permission rather than silently switching clients. Skill metadata improves selection but cannot guarantee another agent's compliance. |
| Ambiguous target | Use filtered `list --json`, then select a full ID. For name selection use `--group`; source selection for `edit`/`duplicate` uses `--from-group`. Do not pick an arbitrary match. |
| Unexpected menu or picker | Use an explicit command and target. Avoid bare invocation, `quick`, `cli`, or `tui` in automation. |
| JSON rejected for connection/transfer/tunnel | Use `--dry-run --json` for preview. Remove both only for an authorized real operation with native output and a suitable process lifecycle. |
| Noninteractive confirmation error | Obtain authorization for the specific operation first, then use its `--yes` flag. Do not enable a global no-confirm policy. |
| Authentication blocks | Native OpenSSH may need an agent, key, password, or terminal. Explain the environment limitation; do not read private keys or circumvent host-key checks. |
| Ping succeeds but SSH fails | TCP reachability does not prove authentication or SSH protocol health. Report the native SSH failure separately. |
| Doctor succeeds but transfer fails | Doctor checks SSH and readable schemas, not SCP availability, remote paths, permissions, or authentication. Check native SCP availability without assuming a supported `scp -V` flag. |
| Invalid/unsupported registry schema | Preserve the file. Explain the recovery message and request a compatible backup or application version; do not delete/reset storage. |
| Import reports registry changed | Another writer changed storage during preview. Reinspect and preview again before applying; do not retry an overwrite blindly. |
| Import rejects unsupported directives | The parser intentionally supports a subset of SSH config. Preserve the original; do not silently discard `Include`, `Match`, patterns, or unsupported options. |
| Session deletion blocked by tunnels | Inspect saved tunnels and ask whether to reassign or delete the references; do not cascade-delete without authorization. |
| Tunnel appears hung | `tunnel run` is a foreground process by design. Stop it using the process/terminal tool that started it when the authorized run ends. |
| Need remote stdout or command execution | No dedicated remote-exec/result command exists. Passthrough is not a supported remote-command contract. Explain the limitation and ask before using another tool. |

## Output and exit status

Most application failures return 1; native processes return their own exit status, and Clap handles argument parsing failures separately. With `--json`, application errors are JSON on stderr, not stdout. Native process output and Clap diagnostics need not be JSON.

A successful preview means local planning succeeded, not that an operation ran. An import preview is printed before applying the write. An interactive cancellation can exit 0 without changing state. Verify the relevant postcondition before claiming a mutation succeeded.

## Capability boundaries

This skill adds no MCP endpoint, daemon, credential manager, batch SSH executor, remote shell sandbox, automatic restart, or remote no-clobber transfer guarantee. Do not suggest nonexistent `exec`, `agent`, `mcp`, `tunnel stop`, or batch flags. Consult installed help and report a capability gap plainly.
