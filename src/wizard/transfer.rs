use super::prompts::{select, text};
use crate::{cli::ScpArgs, service::AppService};
pub fn transfer(service: &AppService, id: &str) -> Result<i32, String> {
    let Some(direction) = select(
        "Transfer direction:",
        vec!["Upload (local to remote)", "Download (remote to local)"],
    )?
    else {
        return Ok(0);
    };
    let download = direction.starts_with("Download");
    let mut files = Vec::new();
    loop {
        let Some(path) = text(
            if download {
                "Remote source path (one path per prompt):"
            } else {
                "Local source path (one path per prompt):"
            },
            "",
        )?
        else {
            return Ok(0);
        };
        files.push(std::path::PathBuf::from(path));
        match select(
            "Add another source?",
            vec!["Continue", "Add another", "Cancel"],
        )? {
            Some("Add another") => {}
            Some("Continue") => break,
            _ => return Ok(0),
        }
    }
    let Some(to) = text(
        if download {
            "Existing local destination directory:"
        } else {
            "Existing remote destination directory:"
        },
        "",
    )?
    else {
        return Ok(0);
    };
    let Some(recursive) = select(
        "Include directories? Recursive SCP may follow symlinks.",
        vec!["Files only", "Recursive", "Cancel"],
    )?
    else {
        return Ok(0);
    };
    if recursive == "Cancel" {
        return Ok(0);
    }
    let args = ScpArgs {
        target: id.to_owned(),
        group: None,
        files,
        to: to.into(),
        download,
        recursive: recursive == "Recursive",
        yes: false,
        dry_run: false,
    };
    let plan = crate::transfer::TransferPlan::new(&service.get(id, None)?, &args)?;
    plan.preview(false)?;
    if !crate::dispatch::confirm(
        "Proceed with transfer and possible overwrite?",
        false,
        false,
    )? {
        return Ok(0);
    }
    let code = plan.execute()?;
    println!("SCP exited with status {code}");
    Ok(code)
}
