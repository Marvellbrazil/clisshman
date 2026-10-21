use super::prompts::{field, optional, select};
use super::{deletion, pick_session};
use crate::{
    cli::{ListArgs, TunnelAddArgs, TunnelEditArgs},
    model::TunnelKind,
    service::AppService,
};
use inquire::Select;
pub fn tunnel_form(
    service: &AppService,
    existing: Option<&crate::model::Tunnel>,
) -> Result<(), String> {
    let name = field(
        "Tunnel name:",
        existing.map(|t| t.name.as_str()).unwrap_or(""),
    )?;
    let Some(session) = pick_session(crate::selection::ranked(
        service.list(&ListArgs::default())?,
        "",
    ))?
    else {
        return Ok(());
    };
    let Some(kind) = select(
        "Forwarding kind:",
        vec!["Local", "Remote", "Dynamic (SOCKS)"],
    )?
    else {
        return Ok(());
    };
    let kind = match kind {
        "Local" => TunnelKind::Local,
        "Remote" => TunnelKind::Remote,
        _ => TunnelKind::Dynamic,
    };
    let bind = field(
        "Bind IP (non-loopback exposes the listener to the network):",
        existing.map(|t| t.bind.as_str()).unwrap_or("127.0.0.1"),
    )?;
    let port = field(
        "Listen port:",
        &existing
            .map(|t| t.listen_port.to_string())
            .unwrap_or_default(),
    )?
    .parse()
    .map_err(|_| "Invalid listen port")?;
    let (destination_host, destination_port) = if kind == TunnelKind::Dynamic {
        (None, None)
    } else {
        (
            Some(field(
                "Destination host:",
                existing
                    .and_then(|t| t.destination_host.as_deref())
                    .unwrap_or("localhost"),
            )?),
            Some(
                field(
                    "Destination port:",
                    &existing
                        .and_then(|t| t.destination_port)
                        .map(|p| p.to_string())
                        .unwrap_or_default(),
                )?
                .parse()
                .map_err(|_| "Invalid destination port")?,
            ),
        )
    };
    let loopback = bind
        .parse::<std::net::IpAddr>()
        .map_err(|_| "Invalid bind IP")?
        .is_loopback();
    let allowed = if loopback {
        false
    } else {
        select(
            "This listener may accept network connections. Allow this binding?",
            vec!["Cancel", "Allow public bind"],
        )? == Some("Allow public bind")
    };
    if !loopback && !allowed {
        return Ok(());
    }
    if let Some(existing) = existing {
        service.edit_tunnel(TunnelEditArgs {
            target: existing.id.clone(),
            name: Some(name),
            session: Some(session),
            kind: Some(kind),
            bind: Some(bind),
            listen_port: Some(port),
            destination_host,
            destination_port,
            allow_public_bind: allowed,
            ..Default::default()
        })?;
    } else {
        service.add_tunnel(TunnelAddArgs {
            name,
            session,
            group: None,
            kind,
            bind,
            listen_port: port,
            destination_host,
            destination_port,
            allow_public_bind: allowed,
        })?;
    }
    Ok(())
}

pub fn tunnels(service: &AppService) -> Result<(), String> {
    loop {
        let Some(action) = select(
            "Tunnels",
            vec!["List", "Add", "Show", "Edit", "Delete", "Run", "Back"],
        )?
        else {
            return Ok(());
        };
        if action == "Back" {
            return Ok(());
        }
        let result = (|| {
            if action == "Add" {
                return tunnel_form(service, None);
            }
            let tunnels = service.storage.load_registry()?.tunnels;
            if action == "List" {
                return crate::output::json(&tunnels);
            }
            if tunnels.is_empty() {
                println!("No tunnels.");
                return Ok(());
            }
            let labels: Vec<_> = tunnels
                .iter()
                .map(|t| {
                    format!(
                        "{} | {} | {:?} {}:{}",
                        t.id, t.name, t.kind, t.bind, t.listen_port
                    )
                })
                .collect();
            let Some(selected) = optional(Select::new("Choose tunnel:", labels.clone()).prompt())?
            else {
                return Ok(());
            };
            let index = labels
                .iter()
                .position(|label| label == &selected)
                .ok_or("Tunnel selection changed")?;
            let tunnel = &tunnels[index];
            match action {
                "Show" => crate::output::json(tunnel)?,
                "Edit" => tunnel_form(service, Some(tunnel))?,
                "Delete" => {
                    if deletion("Delete tunnel?")? {
                        service.delete_tunnel(&tunnel.id)?;
                    }
                }
                "Run" => {
                    println!(
                        "Running tunnel {} on {}:{}, Ctrl+C stops it.",
                        tunnel.name, tunnel.bind, tunnel.listen_port
                    );
                    let code = service.run_tunnel_returning(&tunnel.id)?;
                    println!("SSH exited with status {code}");
                }
                _ => {}
            }
            Ok(())
        })();
        if let Err(error) = result {
            println!("{error}");
        }
    }
}
