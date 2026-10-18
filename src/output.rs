use crate::model::Session;
use comfy_table::{
    presets::{NOTHING, UTF8_FULL},
    Table,
};
use serde::Serialize;
use std::io::IsTerminal;

pub fn json(value: &impl Serialize) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|_| "Cannot serialize output")?
    );
    Ok(())
}
pub fn sessions(sessions: &[Session], as_json: bool) -> Result<(), String> {
    if as_json {
        return json(&sessions);
    }
    let mut table = Table::new();
    table.load_preset(if std::io::stdout().is_terminal() {
        UTF8_FULL
    } else {
        NOTHING
    });
    table.set_header([
        "ID",
        "Group",
        "Name",
        "Hostname",
        "Username",
        "Port",
        "Tags",
        "Favorite",
        "Last attempt",
        "Created At",
        "Updated At",
    ]);
    for s in sessions {
        let id = if uuid::Uuid::parse_str(&s.id).is_ok() {
            s.id[..8].to_owned()
        } else {
            s.id.clone()
        };
        table.add_row([
            id,
            s.group.clone().unwrap_or_default(),
            s.name.clone(),
            s.host.clone(),
            s.user.clone(),
            s.port.to_string(),
            s.tags.join(", "),
            s.favorite.to_string(),
            s.last_connect_attempt_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
            s.created_at.to_rfc3339(),
            s.updated_at.to_rfc3339(),
        ]);
    }
    println!("{table}");
    Ok(())
}
pub fn session(value: &Session, as_json: bool) -> Result<(), String> {
    if as_json {
        json(value)
    } else {
        let object = serde_json::to_value(value).map_err(|_| "Cannot format session")?;
        if let Some(fields) = object.as_object() {
            for (key, value) in fields {
                println!("{key}: {value}");
            }
        }
        Ok(())
    }
}
pub fn command(args: &[String], as_json: bool) -> Result<(), String> {
    native_command("ssh", args, as_json)
}
pub fn native_command(program: &str, args: &[String], as_json: bool) -> Result<(), String> {
    if as_json {
        return json(&serde_json::json!({"program":program,"args":args}));
    }
    let quote = |arg: &String| {
        if arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-./:@=,".contains(c))
        {
            arg.clone()
        } else if cfg!(windows) {
            format!("'{}'", arg.replace('\'', "''"))
        } else {
            format!("'{}'", arg.replace('\'', "'\\''"))
        }
    };
    println!(
        "{program} {}",
        args.iter().map(quote).collect::<Vec<_>>().join(" ")
    );
    Ok(())
}
