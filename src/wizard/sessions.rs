use super::prompts::field;
use crate::{
    cli::{AddArgs, EditArgs},
    model::Session,
    service::AppService,
};
pub fn session_form(
    service: &AppService,
    source: Option<&Session>,
    duplicate: bool,
) -> Result<(), String> {
    let config = service.storage.load_config()?;
    let get = |f: fn(&Session) -> &str| source.map(f).unwrap_or("");
    let name = field(
        "Name (empty generates user@host for new sessions):",
        if duplicate { "" } else { get(|s| &s.name) },
    )?;
    if duplicate && name.trim().is_empty() {
        return Err("Duplicate requires a new name".into());
    }
    let host = field("Hostname / IP:", get(|s| &s.host))?;
    let user = field("Username:", get(|s| &s.user))?;
    let port = field(
        "Port:",
        &source
            .map(|s| s.port)
            .unwrap_or(config.default_port)
            .to_string(),
    )?
    .parse()
    .map_err(|_| "Invalid port")?;
    let group = field(
        "Group (empty clears):",
        source.and_then(|s| s.group.as_deref()).unwrap_or(""),
    )?;
    let identity = field(
        "Identity file (empty clears):",
        source
            .and_then(|s| s.identity_file.as_deref())
            .unwrap_or(""),
    )?;
    let jump = field(
        "Jump host (empty clears):",
        source.and_then(|s| s.jump_host.as_deref()).unwrap_or(""),
    )?;
    let description = field(
        "Description (empty clears):",
        source.and_then(|s| s.description.as_deref()).unwrap_or(""),
    )?;
    let tags = field(
        "Tags (comma-separated, empty clears):",
        &source.map(|s| s.tags.join(", ")).unwrap_or_default(),
    )?;
    let option = |s: String| if s.trim().is_empty() { None } else { Some(s) };
    if let Some(source) = source.filter(|_| !duplicate) {
        service.edit(EditArgs {
            target: source.id.clone(),
            name: Some(name),
            host: Some(host),
            username: Some(user),
            port: Some(port),
            clear_group: group.trim().is_empty(),
            group: option(group),
            clear_identity: identity.trim().is_empty(),
            identity_file: option(identity),
            clear_jump: jump.trim().is_empty(),
            jump_host: option(jump),
            clear_description: description.trim().is_empty(),
            description: option(description),
            clear_tags: tags.trim().is_empty(),
            tags: if tags.trim().is_empty() {
                vec![]
            } else {
                vec![tags]
            },
            ..Default::default()
        })?;
    } else {
        service.add(AddArgs {
            name: option(name),
            host: Some(host),
            username: Some(user),
            port: Some(port),
            group: option(group),
            identity_file: option(identity),
            jump_host: option(jump),
            description: option(description),
            tags: if tags.trim().is_empty() {
                vec![]
            } else {
                vec![tags]
            },
            ..Default::default()
        })?;
    }
    Ok(())
}
