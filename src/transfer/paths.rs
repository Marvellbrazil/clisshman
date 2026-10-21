use crate::model::Session;
use directories::BaseDirs;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
pub(super) fn expand_home(value: &Path) -> Result<PathBuf, String> {
    if let Some(suffix) = value.to_str().and_then(|v| v.strip_prefix("~/")) {
        let dirs = BaseDirs::new().ok_or("Cannot determine the home directory")?;
        return Ok(dirs.home_dir().join(suffix));
    }
    Ok(value.to_owned())
}

pub(super) fn local_path(value: &Path) -> Result<PathBuf, String> {
    let expanded = expand_home(value)?;
    if expanded.as_os_str().is_empty() {
        return Err("Local path must not be empty".into());
    }
    let path = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()
            .map_err(|_| "Cannot determine current directory")?
            .join(expanded)
    };
    let text = path.to_str().ok_or("SCP requires Unicode paths")?;
    if text.chars().any(char::is_control) {
        return Err("Paths must not contain control characters".into());
    }
    Ok(path)
}

pub(super) fn local_operand(value: &Path) -> Result<OsString, String> {
    let text = value.to_str().ok_or("SCP requires Unicode paths")?;
    #[cfg(windows)]
    {
        if text.starts_with("\\\\") {
            return Err(
                "UNC source/destination paths are not supported by this SCP wrapper".into(),
            );
        }
        Ok(OsString::from(text.replace('\\', "/")))
    }
    #[cfg(not(windows))]
    {
        Ok(OsString::from(text))
    }
}

pub(super) fn remote_operand(
    session: &Session,
    path: &Path,
    directory: bool,
) -> Result<OsString, String> {
    let text = path.to_str().ok_or("Remote path must be Unicode")?;
    if text.trim().is_empty() || text.chars().any(char::is_control) {
        return Err("Remote path must be nonempty and contain no control characters".into());
    }
    if text.contains([
        '*', '?', '[', ']', '{', '}', '\\', '`', '$', ';', '&', '|', '<', '>', '\'', '"', '!', '#',
        '(', ')',
    ]) {
        return Err("Remote paths must be literal: wildcards, shell syntax, and backslashes are not supported".into());
    }
    let mut path = if text.starts_with('/') || text.starts_with("~/") {
        text.to_owned()
    } else {
        format!("./{text}")
    };
    if directory && !path.ends_with('/') {
        path.push('/');
    }
    let host = if session.host.contains(':') {
        format!("[{}]", session.host)
    } else {
        session.host.clone()
    };
    Ok(OsString::from(format!("{}@{host}:{path}", session.user)))
}
