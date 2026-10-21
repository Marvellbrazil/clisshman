use crate::{cli::ListArgs, selection, service::AppService};
use std::io::{self, IsTerminal, Write};
pub fn interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

pub fn confirm(message: &str, yes: bool, as_json: bool) -> Result<bool, String> {
    if yes {
        return Ok(true);
    }
    if !interactive() || as_json {
        return Err("Confirmation requires a terminal; pass --yes explicitly".into());
    }
    print!("{message} [y/N]: ");
    io::stdout()
        .flush()
        .map_err(|_| "Cannot display confirmation")?;
    let mut value = String::new();
    io::stdin()
        .read_line(&mut value)
        .map_err(|_| "Cannot read confirmation")?;
    Ok(value.trim().eq_ignore_ascii_case("y") || value.trim().eq_ignore_ascii_case("yes"))
}

pub fn target(
    service: &AppService,
    value: Option<&str>,
    group: Option<&str>,
    as_json: bool,
) -> Result<Option<String>, String> {
    let registry = service.storage.load_registry()?;
    let candidates = if let Some(value) = value {
        selection::candidates(&registry, value, group)
    } else {
        selection::ranked(
            selection::filtered(
                &registry,
                &ListArgs {
                    group: group.map(str::to_owned),
                    ..Default::default()
                },
            ),
            "",
        )
    };
    if candidates.is_empty() {
        return Err("No matching sessions".into());
    }
    if value.is_some() && candidates.len() == 1 {
        return Ok(Some(candidates[0].id.clone()));
    }
    if !interactive() || as_json {
        return Err(format!(
            "Specify an unambiguous target ID. Candidates: {}",
            candidates
                .iter()
                .map(|s| format!("{} ({})", s.id, s.name))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    crate::wizard::pick_session(candidates)
}

pub(super) fn require_terminal(json: bool) -> Result<(), String> {
    if interactive() && !json {
        Ok(())
    } else {
        Err("This mode requires an interactive terminal without --json".into())
    }
}
