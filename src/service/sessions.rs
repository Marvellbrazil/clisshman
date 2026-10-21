use super::{resolve_session, resolve_session_index, AppService};
use crate::{
    cli::{AddArgs, DuplicateArgs, EditArgs, ListArgs},
    model::{generate_id, normalize_tags, Session},
    validation,
};
use chrono::Utc;
impl AppService {
    pub fn list(&self, filters: &ListArgs) -> Result<Vec<Session>, String> {
        Ok(crate::selection::filtered(
            &self.storage.load_registry()?,
            filters,
        ))
    }
    pub fn get(&self, target: &str, group: Option<&str>) -> Result<Session, String> {
        Ok(resolve_session(&self.storage.load_registry()?, target, group)?.clone())
    }
    pub fn add(&self, args: AddArgs) -> Result<Session, String> {
        let config = self.storage.load_config()?;
        let parsed = args
            .target
            .as_deref()
            .map(validation::parse_target)
            .transpose()?;
        let user = args
            .username
            .or_else(|| parsed.as_ref().and_then(|v| v.0.clone()))
            .ok_or("Username is required (-u or user@host)")?;
        let host = args
            .host
            .or_else(|| parsed.as_ref().map(|v| v.1.clone()))
            .ok_or("Hostname is required (-H or user@host)")?;
        let port = args
            .port
            .or_else(|| parsed.as_ref().and_then(|v| v.2))
            .unwrap_or(config.default_port);
        check_tags(&args.tags)?;
        let now = Utc::now();
        let mut session = Session {
            id: String::new(),
            name: args.name.unwrap_or_else(|| format!("{user}@{host}")),
            group: args.group,
            host,
            user,
            port,
            identity_file: args.identity_file,
            jump_host: args.jump_host,
            description: args.description,
            tags: normalize_tags(&args.tags),
            favorite: false,
            last_connect_attempt_at: None,
            created_at: now,
            updated_at: now,
        };
        validation::session(&mut session)?;
        self.storage.update_registry(|registry| {
            session.id = generate_id(config.id_type, &mut registry.next_id)?;
            registry.sessions.push(session.clone());
            Ok(session)
        })
    }
    pub fn edit(&self, args: EditArgs) -> Result<Session, String> {
        check_tags(&args.tags)?;
        if (args.clear_group && args.group.is_some())
            || (args.clear_identity && args.identity_file.is_some())
            || (args.clear_jump && args.jump_host.is_some())
            || (args.clear_description && args.description.is_some())
            || (args.clear_tags && !args.tags.is_empty())
        {
            return Err("Cannot set and clear the same field".into());
        }
        self.storage.update_registry(|registry| {
            let index = resolve_session_index(registry, &args.target, args.from_group.as_deref())?;
            let s = &mut registry.sessions[index];
            if let Some(value) = args.name {
                s.name = value;
            }
            if let Some(value) = args.group {
                s.group = Some(value);
            }
            if let Some(value) = args.host {
                s.host = value;
            }
            if let Some(value) = args.username {
                s.user = value;
            }
            if let Some(value) = args.port {
                s.port = value;
            }
            if let Some(value) = args.identity_file {
                s.identity_file = Some(value);
            }
            if let Some(value) = args.jump_host {
                s.jump_host = Some(value);
            }
            if let Some(value) = args.description {
                s.description = Some(value);
            }
            if !args.tags.is_empty() {
                s.tags = normalize_tags(&args.tags);
            }
            if args.clear_tags {
                s.tags.clear();
            }
            if args.clear_group {
                s.group = None;
            }
            if args.clear_identity {
                s.identity_file = None;
            }
            if args.clear_jump {
                s.jump_host = None;
            }
            if args.clear_description {
                s.description = None;
            }
            s.updated_at = Utc::now();
            validation::session(s)?;
            Ok(s.clone())
        })
    }
    pub fn duplicate(&self, args: DuplicateArgs) -> Result<Session, String> {
        let config = self.storage.load_config()?;
        self.storage.update_registry(|registry| {
            let mut copy =
                resolve_session(registry, &args.target, args.from_group.as_deref())?.clone();
            copy.name = args.name;
            if let Some(value) = args.group {
                copy.group = Some(value);
            }
            if let Some(value) = args.host {
                copy.host = value;
            }
            if let Some(value) = args.username {
                copy.user = value;
            }
            if let Some(value) = args.port {
                copy.port = value;
            }
            copy.id = generate_id(config.id_type, &mut registry.next_id)?;
            copy.created_at = Utc::now();
            copy.updated_at = copy.created_at;
            copy.favorite = false;
            copy.last_connect_attempt_at = None;
            validation::session(&mut copy)?;
            registry.sessions.push(copy.clone());
            Ok(copy)
        })
    }
    pub fn delete(&self, target: &str, group: Option<&str>) -> Result<Session, String> {
        self.storage.update_registry(|registry| {
            let index = resolve_session_index(registry, target, group)?;
            let id = &registry.sessions[index].id;
            let dependencies: Vec<_> = registry
                .tunnels
                .iter()
                .filter(|t| &t.session_id == id)
                .map(|t| t.name.as_str())
                .collect();
            if !dependencies.is_empty() {
                return Err(format!(
                    "Session is used by tunnels: {}. Remove or move them first.",
                    dependencies.join(", ")
                ));
            }
            Ok(registry.sessions.remove(index))
        })
    }
    pub fn favorite(
        &self,
        target: &str,
        group: Option<&str>,
        favorite: bool,
    ) -> Result<Session, String> {
        self.storage.update_registry(|registry| {
            let index = resolve_session_index(registry, target, group)?;
            let s = &mut registry.sessions[index];
            s.favorite = favorite;
            s.updated_at = Utc::now();
            Ok(s.clone())
        })
    }
    pub fn record_attempt(&self, id: &str) -> Result<Session, String> {
        self.storage.update_registry(|registry| {
            let index = resolve_session_index(registry, id, None)?;
            registry.sessions[index].last_connect_attempt_at = Some(Utc::now());
            Ok(registry.sessions[index].clone())
        })
    }
}
fn check_tags(tags: &[String]) -> Result<(), String> {
    if !tags.is_empty() && normalize_tags(tags).is_empty() {
        Err("Tags must not be entirely empty; use --clear-tags".into())
    } else {
        Ok(())
    }
}
