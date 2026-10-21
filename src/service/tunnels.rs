use super::{resolve_session, AppService};
use crate::{
    cli::{TunnelAddArgs, TunnelEditArgs},
    model::{generate_id, Tunnel, TunnelKind},
    validation,
};
use chrono::Utc;
impl AppService {
    pub fn add_tunnel(&self, args: TunnelAddArgs) -> Result<Tunnel, String> {
        let config = self.storage.load_config()?;
        self.storage.update_registry(|registry| {
            let session_id = resolve_session(registry, &args.session, args.group.as_deref())?
                .id
                .clone();
            let now = Utc::now();
            let tunnel = Tunnel {
                id: generate_id(config.id_type, &mut registry.next_tunnel_id)?,
                name: args.name.trim().to_owned(),
                session_id,
                kind: args.kind,
                bind: args.bind,
                listen_port: args.listen_port,
                destination_host: args.destination_host,
                destination_port: args.destination_port,
                allow_public_bind: args.allow_public_bind,
                created_at: now,
                updated_at: now,
            };
            validation::tunnel(&tunnel)?;
            registry.tunnels.push(tunnel.clone());
            Ok(tunnel)
        })
    }
    pub fn edit_tunnel(&self, args: TunnelEditArgs) -> Result<Tunnel, String> {
        self.storage.update_registry(|registry| {
            let index = crate::selection::resolve_tunnel_index(registry, &args.target)?;
            let session_id = args
                .session
                .as_deref()
                .map(|target| {
                    resolve_session(registry, target, args.group.as_deref()).map(|s| s.id.clone())
                })
                .transpose()?;
            let t = &mut registry.tunnels[index];
            if let Some(value) = args.name {
                t.name = value.trim().to_owned();
            }
            if let Some(value) = session_id {
                t.session_id = value;
            }
            if let Some(value) = args.kind {
                if value == TunnelKind::Dynamic
                    && (args.destination_host.is_some() || args.destination_port.is_some())
                {
                    return Err("Dynamic tunnels do not accept destination fields".into());
                }
                t.kind = value;
                if value == TunnelKind::Dynamic {
                    t.destination_host = None;
                    t.destination_port = None;
                }
            }
            if let Some(value) = args.bind {
                if value != t.bind {
                    t.allow_public_bind = false;
                }
                t.bind = value;
            }
            if let Some(value) = args.listen_port {
                t.listen_port = value;
            }
            if let Some(value) = args.destination_host {
                t.destination_host = Some(value);
            }
            if let Some(value) = args.destination_port {
                t.destination_port = Some(value);
            }
            if args.allow_public_bind {
                t.allow_public_bind = true;
            }
            t.updated_at = Utc::now();
            validation::tunnel(t)?;
            Ok(t.clone())
        })
    }
    pub fn get_tunnel(&self, target: &str) -> Result<Tunnel, String> {
        let registry = self.storage.load_registry()?;
        Ok(registry.tunnels[crate::selection::resolve_tunnel_index(&registry, target)?].clone())
    }
    pub fn delete_tunnel(&self, target: &str) -> Result<Tunnel, String> {
        self.storage.update_registry(|registry| {
            let index = crate::selection::resolve_tunnel_index(registry, target)?;
            Ok(registry.tunnels.remove(index))
        })
    }
    pub fn run_tunnel_returning(&self, target: &str) -> Result<i32, String> {
        let tunnel = self.get_tunnel(target)?;
        let session = self.record_attempt(&tunnel.session_id)?;
        crate::connect::SshConnection::execute_returning(&session, &tunnel_args(&tunnel)?)
    }
}
pub fn tunnel_args(tunnel: &Tunnel) -> Result<Vec<String>, String> {
    validation::tunnel(tunnel)?;
    let address = |value: &str| {
        if value.contains(':') {
            format!("[{value}]")
        } else {
            value.to_owned()
        }
    };
    let mut value = format!("{}:{}", address(&tunnel.bind), tunnel.listen_port);
    if tunnel.kind != TunnelKind::Dynamic {
        value.push_str(&format!(
            ":{}:{}",
            address(
                tunnel
                    .destination_host
                    .as_deref()
                    .ok_or("Missing destination")?
            ),
            tunnel.destination_port.ok_or("Missing destination port")?
        ));
    }
    Ok(vec![
        "-N".into(),
        "-o".into(),
        "ExitOnForwardFailure=yes".into(),
        match tunnel.kind {
            TunnelKind::Local => "-L",
            TunnelKind::Remote => "-R",
            TunnelKind::Dynamic => "-D",
        }
        .into(),
        value,
    ])
}
