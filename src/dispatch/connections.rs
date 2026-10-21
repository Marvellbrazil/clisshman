use super::{confirm, interactive, target};
use crate::{
    cli::{ConnectArgs, PingArgs, QuickArgs, ScpArgs},
    output, selection,
    service::AppService,
};
pub(super) fn quick(args: QuickArgs, service: &AppService, json: bool) -> Result<i32, String> {
    if !interactive() || json {
        return Err("quick requires an interactive terminal".into());
    }
    let sessions = selection::ranked(
        service.list(&args.filters)?,
        args.query.as_deref().unwrap_or(""),
    );
    if let Some(id) = crate::wizard::pick_session(sessions)? {
        return service.connect_returning(&id);
    }
    Ok(0)
}
pub(super) fn connect(args: ConnectArgs, service: &AppService, json: bool) -> Result<i32, String> {
    if args.target.is_none() && args.dry_run {
        return Err("--dry-run requires a target".into());
    }
    let Some(id) = target(service, args.target.as_deref(), args.group.as_deref(), json)? else {
        return Ok(0);
    };
    if args.dry_run {
        output::command(
            &crate::connect::SshConnection::build_args(&service.get(&id, None)?, &args.passthrough),
            json,
        )?;
    } else {
        if json {
            return Err("Use --dry-run with --json for connect".into());
        }
        let session = service.record_attempt(&id)?;
        return crate::connect::SshConnection::execute(&session, &args.passthrough);
    }
    Ok(0)
}
pub(super) fn ping(args: PingArgs, service: &AppService, json: bool) -> Result<i32, String> {
    if let Some(id) = target(service, Some(&args.target), args.group.as_deref(), json)? {
        let result = service.ping(&id, args.icmp, args.timeout)?;
        if json {
            output::json(&result)?;
        } else {
            println!("{}: {}", result.target, result.message);
        }
        return Ok(if result.success { 0 } else { 1 });
    }
    Ok(0)
}
pub(super) fn scp(args: ScpArgs, service: &AppService, json: bool) -> Result<i32, String> {
    let Some(id) = target(service, Some(&args.target), args.group.as_deref(), json)? else {
        return Ok(0);
    };
    let plan = crate::transfer::TransferPlan::new(&service.get(&id, None)?, &args)?;
    if json && !args.dry_run {
        return Err("Use --dry-run with --json for SCP; native progress is not JSON".into());
    }
    plan.preview(json)?;
    if args.dry_run {
        return Ok(0);
    }
    if confirm(
        "Proceed with transfer and possible overwrite?",
        args.yes,
        json,
    )? {
        return plan.execute();
    }
    Ok(0)
}
