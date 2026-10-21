use super::{confirm, interactive, target};
use crate::{cli::*, model::Registry, output, selection, service::AppService};
pub(super) fn add(args: AddArgs, service: &AppService, json: bool) -> Result<i32, String> {
    output::session(&service.add(args)?, json)?;
    Ok(0)
}
pub(super) fn edit(mut args: EditArgs, service: &AppService, json: bool) -> Result<i32, String> {
    let Some(id) = target(
        service,
        Some(&args.target),
        args.from_group.as_deref(),
        json,
    )?
    else {
        return Ok(0);
    };
    args.target = id;
    args.from_group = None;
    output::session(&service.edit(args)?, json)?;
    Ok(0)
}
pub(super) fn duplicate(
    mut args: DuplicateArgs,
    service: &AppService,
    json: bool,
) -> Result<i32, String> {
    let Some(id) = target(
        service,
        Some(&args.target),
        args.from_group.as_deref(),
        json,
    )?
    else {
        return Ok(0);
    };
    args.target = id;
    args.from_group = None;
    output::session(&service.duplicate(args)?, json)?;
    Ok(0)
}
pub(super) fn list(args: ListArgs, service: &AppService, json: bool) -> Result<i32, String> {
    output::sessions(&service.list(&args)?, json)?;
    Ok(0)
}
pub(super) fn show(args: ShowArgs, service: &AppService, json: bool) -> Result<i32, String> {
    let candidates = if let Some(value) = args.target {
        let Some(id) = target(service, Some(&value), args.filters.group.as_deref(), json)? else {
            return Ok(0);
        };
        let s = service.get(&id, None)?;
        let mut registry = Registry::default();
        registry.sessions.push(s);
        selection::filtered(&registry, &args.filters)
    } else {
        service.list(&args.filters)?
    };
    let selected = if candidates.len() == 1 {
        Some(candidates[0].id.clone())
    } else if interactive() && !json {
        crate::wizard::pick_session(candidates)?
    } else {
        return Err("show requires exactly one match; use list or an exact ID".into());
    };
    if let Some(id) = selected {
        output::session(&service.get(&id, None)?, json)?;
    }
    Ok(0)
}
pub(super) fn delete(args: DeleteArgs, service: &AppService, json: bool) -> Result<i32, String> {
    let Some(id) = target(service, Some(&args.target), args.group.as_deref(), json)? else {
        return Ok(0);
    };
    let config = service.storage.load_config()?;
    let yes = args.yes || (interactive() && !json && !config.confirm_delete);
    if confirm(
        &format!("Delete session '{}' ?", service.get(&id, None)?.name),
        yes,
        json,
    )? {
        output::session(&service.delete(&id, None)?, json)?;
    }
    Ok(0)
}
pub(super) fn favorite(
    args: FavoriteArgs,
    service: &AppService,
    json: bool,
) -> Result<i32, String> {
    if let Some(id) = target(service, Some(&args.target), args.group.as_deref(), json)? {
        output::session(&service.favorite(&id, None, !args.unset)?, json)?;
    }
    Ok(0)
}
pub(super) fn config(args: ConfigArgs, service: &AppService, _json: bool) -> Result<i32, String> {
    let config = match (args.key.as_deref(), args.value.as_deref()) {
        (Some(key), Some(value)) => service.set_config(key, value)?,
        (None, Some(_)) => return Err("Config key is required".into()),
        _ => service.storage.load_config()?,
    };
    if let (Some(key), None) = (args.key, args.value) {
        let value = serde_json::to_value(&config).map_err(|_| "Cannot format config")?;
        output::json(value.get(key.to_lowercase()).ok_or("Unknown config key")?)?;
    } else {
        output::json(&config)?;
    }
    Ok(0)
}
