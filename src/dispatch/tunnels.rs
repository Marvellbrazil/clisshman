use super::confirm;
use crate::{cli::TunnelCommands, output, service::AppService};
pub(super) fn tunnels(
    command: TunnelCommands,
    service: &AppService,
    json: bool,
) -> Result<i32, String> {
    match command {
        TunnelCommands::Add(args) => output::json(&service.add_tunnel(args)?)?,
        TunnelCommands::List => output::json(&service.storage.load_registry()?.tunnels)?,
        TunnelCommands::Show(args) => output::json(&service.get_tunnel(&args.target)?)?,
        TunnelCommands::Edit(args) => output::json(&service.edit_tunnel(args)?)?,
        TunnelCommands::Delete(args) => {
            if confirm("Delete tunnel?", args.yes, json)? {
                output::json(&service.delete_tunnel(&args.target)?)?;
            }
        }
        TunnelCommands::Run(args) => {
            let tunnel = service.get_tunnel(&args.target)?;
            let extra = crate::service::tunnel_args(&tunnel)?;
            let session = service.get(&tunnel.session_id, None)?;
            if args.dry_run {
                output::command(
                    &crate::connect::SshConnection::build_args(&session, &extra),
                    json,
                )?;
            } else {
                if json {
                    return Err("Use --dry-run with --json for tunnel run".into());
                }
                eprintln!(
                    "Tunnel '{}' binds {}:{} ({:?}); Ctrl+C stops it.",
                    tunnel.name, tunnel.bind, tunnel.listen_port, tunnel.kind
                );
                let session = service.record_attempt(&session.id)?;
                return crate::connect::SshConnection::execute(&session, &extra);
            }
        }
    }
    Ok(0)
}
