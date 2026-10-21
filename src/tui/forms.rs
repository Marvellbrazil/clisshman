use super::render::kind_label;
use super::FormKind;
use crate::cli::{AddArgs, DuplicateArgs, EditArgs, TunnelAddArgs};
use crate::model::{Session, Tunnel, TunnelKind};

#[derive(Debug, Clone)]
pub struct Form {
    pub kind: FormKind,
    pub target: Option<String>,
    pub fields: Vec<(&'static str, String)>,
    pub focused: usize,
    pub error: Option<String>,
}

impl Form {
    pub(super) fn session(kind: FormKind, session: Option<&Session>, port: u16) -> Self {
        let mut fields = vec![
            ("Name", session.map(|s| s.name.clone()).unwrap_or_default()),
            (
                "Group",
                session.and_then(|s| s.group.clone()).unwrap_or_default(),
            ),
            ("Host", session.map(|s| s.host.clone()).unwrap_or_default()),
            (
                "Username",
                session.map(|s| s.user.clone()).unwrap_or_default(),
            ),
            ("Port", session.map(|s| s.port).unwrap_or(port).to_string()),
        ];
        if kind == FormKind::DuplicateSession {
            fields[0].1.push_str("-copy");
            fields[1].0 = "Group (blank keeps source)";
        } else {
            fields.extend([
                (
                    "Identity file",
                    session
                        .and_then(|s| s.identity_file.clone())
                        .unwrap_or_default(),
                ),
                (
                    "Jump host",
                    session
                        .and_then(|s| s.jump_host.clone())
                        .unwrap_or_default(),
                ),
                (
                    "Description",
                    session
                        .and_then(|s| s.description.clone())
                        .unwrap_or_default(),
                ),
                (
                    "Tags (comma separated)",
                    session.map(|s| s.tags.join(", ")).unwrap_or_default(),
                ),
            ]);
        }
        Self {
            kind,
            target: session.map(|s| s.id.clone()),
            fields,
            focused: 0,
            error: None,
        }
    }

    pub(super) fn tunnel(tunnel: Option<&Tunnel>, session: Option<&str>) -> Self {
        Self {
            kind: if tunnel.is_some() {
                FormKind::EditTunnel
            } else {
                FormKind::AddTunnel
            },
            target: tunnel.map(|t| t.id.clone()),
            fields: vec![
                ("Name", tunnel.map(|t| t.name.clone()).unwrap_or_default()),
                (
                    "Session ID",
                    tunnel
                        .map(|t| t.session_id.clone())
                        .or_else(|| session.map(str::to_owned))
                        .unwrap_or_default(),
                ),
                (
                    "Kind (local/remote/dynamic)",
                    tunnel
                        .map(|t| kind_label(t.kind).to_owned())
                        .unwrap_or_else(|| "local".into()),
                ),
                (
                    "Bind",
                    tunnel
                        .map(|t| t.bind.clone())
                        .unwrap_or_else(|| "127.0.0.1".into()),
                ),
                (
                    "Listen port",
                    tunnel
                        .map(|t| t.listen_port.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "Destination host",
                    tunnel
                        .and_then(|t| t.destination_host.clone())
                        .unwrap_or_default(),
                ),
                (
                    "Destination port",
                    tunnel
                        .and_then(|t| t.destination_port)
                        .map(|p| p.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "Allow public bind (true/false)",
                    tunnel
                        .map(|t| t.allow_public_bind)
                        .unwrap_or(false)
                        .to_string(),
                ),
            ],
            focused: 0,
            error: None,
        }
    }

    pub(super) fn value(&self, index: usize) -> String {
        self.fields[index].1.trim().to_owned()
    }
    pub(super) fn optional(&self, index: usize) -> Option<String> {
        let value = self.value(index);
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }
    pub(super) fn required(&self, index: usize) -> Result<String, String> {
        self.optional(index)
            .ok_or_else(|| format!("{} is required", self.fields[index].0))
    }
    pub(super) fn port(&self, index: usize) -> Result<u16, String> {
        self.value(index)
            .parse::<u16>()
            .ok()
            .filter(|p| *p > 0)
            .ok_or_else(|| format!("{} must be between 1 and 65535", self.fields[index].0))
    }
    pub(super) fn target(&self) -> Result<String, String> {
        self.target.clone().ok_or_else(|| "No item selected".into())
    }
    pub(super) fn tags(&self) -> Vec<String> {
        self.value(8)
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
            .collect()
    }
    pub(super) fn add_args(&self) -> Result<AddArgs, String> {
        Ok(AddArgs {
            name: Some(self.required(0)?),
            group: self.optional(1),
            host: Some(self.required(2)?),
            username: Some(self.required(3)?),
            port: Some(self.port(4)?),
            identity_file: self.optional(5),
            jump_host: self.optional(6),
            description: self.optional(7),
            tags: self.tags(),
            ..AddArgs::default()
        })
    }
    pub(super) fn edit_args(&self) -> Result<EditArgs, String> {
        Ok(EditArgs {
            target: self.target()?,
            name: Some(self.required(0)?),
            group: self.optional(1),
            host: Some(self.required(2)?),
            username: Some(self.required(3)?),
            port: Some(self.port(4)?),
            identity_file: self.optional(5),
            jump_host: self.optional(6),
            description: self.optional(7),
            tags: self.tags(),
            clear_group: self.optional(1).is_none(),
            clear_identity: self.optional(5).is_none(),
            clear_jump: self.optional(6).is_none(),
            clear_description: self.optional(7).is_none(),
            clear_tags: self.tags().is_empty(),
            ..EditArgs::default()
        })
    }
    pub(super) fn duplicate_args(&self) -> Result<DuplicateArgs, String> {
        Ok(DuplicateArgs {
            target: self.target()?,
            name: self.required(0)?,
            group: self.optional(1),
            host: Some(self.required(2)?),
            username: Some(self.required(3)?),
            port: Some(self.port(4)?),
            ..DuplicateArgs::default()
        })
    }
    pub(super) fn tunnel_args(&self) -> Result<TunnelAddArgs, String> {
        let kind = match self.value(2).to_lowercase().as_str() {
            "local" => TunnelKind::Local,
            "remote" => TunnelKind::Remote,
            "dynamic" => TunnelKind::Dynamic,
            _ => return Err("Kind must be local, remote, or dynamic".into()),
        };
        let allow_public_bind = match self.value(7).to_lowercase().as_str() {
            "true" => true,
            "false" => false,
            _ => return Err("Allow public bind must be true or false".into()),
        };
        if kind == TunnelKind::Dynamic && (self.optional(5).is_some() || self.optional(6).is_some())
        {
            return Err("Clear destination fields for a dynamic tunnel".into());
        }
        Ok(TunnelAddArgs {
            name: self.required(0)?,
            session: self.required(1)?,
            group: None,
            kind,
            bind: self.required(3)?,
            listen_port: self.port(4)?,
            destination_host: if kind == TunnelKind::Dynamic {
                None
            } else {
                Some(self.required(5)?)
            },
            destination_port: if kind == TunnelKind::Dynamic {
                None
            } else {
                Some(self.port(6)?)
            },
            allow_public_bind,
        })
    }
}
