use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use directories::BaseDirs;

use crate::{cli::ScpArgs, model::Session};

#[derive(Debug)]
pub struct TransferPlan {
    pub args: Vec<OsString>,
    pub download: bool,
}

fn expand_home(value: &Path) -> Result<PathBuf, String> {
    if let Some(suffix) = value.to_str().and_then(|v| v.strip_prefix("~/")) {
        let dirs = BaseDirs::new().ok_or("Cannot determine the home directory")?;
        return Ok(dirs.home_dir().join(suffix));
    }
    Ok(value.to_owned())
}

fn local_path(value: &Path) -> Result<PathBuf, String> {
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

fn local_operand(value: &Path) -> Result<OsString, String> {
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

fn remote_operand(session: &Session, path: &Path, directory: bool) -> Result<OsString, String> {
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

impl TransferPlan {
    pub fn new(session: &Session, request: &ScpArgs) -> Result<Self, String> {
        if request.files.is_empty() {
            return Err("At least one --file is required".into());
        }
        let mut args = vec![OsString::from("-P"), session.port.to_string().into()];
        if let Some(identity) = &session.identity_file {
            args.push("-i".into());
            args.push(expand_home(Path::new(identity))?.into_os_string());
        }
        if let Some(jump) = &session.jump_host {
            args.push("-J".into());
            args.push(jump.into());
        }
        if request.recursive {
            args.push("-r".into());
        }
        args.push("--".into());
        if request.download {
            for file in &request.files {
                args.push(remote_operand(session, file, false)?);
            }
            let destination = local_path(&request.to)?;
            if !destination.is_dir() {
                return Err("Download destination must be an existing local directory".into());
            }
            args.push(local_operand(&destination)?);
        } else {
            for file in &request.files {
                let path = local_path(file)?;
                let metadata = std::fs::metadata(&path)
                    .map_err(|_| format!("Cannot read local source: {}", file.display()))?;
                if metadata.is_dir() && !request.recursive {
                    return Err(
                        "Directory sources require --recursive; SCP may follow symlinks".into(),
                    );
                }
                if !metadata.is_file() && !metadata.is_dir() {
                    return Err("Source must be a regular file or directory".into());
                }
                args.push(local_operand(&path)?);
            }
            args.push(remote_operand(session, &request.to, true)?);
        }
        Ok(Self {
            args,
            download: request.download,
        })
    }

    pub fn preview(&self, as_json: bool) -> Result<(), String> {
        if as_json {
            return crate::output::json(&serde_json::json!({
                "program":"scp", "args":self.args.iter().map(|v|v.to_string_lossy()).collect::<Vec<_>>(),
                "direction":if self.download {"download"} else {"upload"},
                "may_overwrite":true
            }));
        }
        let args = self
            .args
            .iter()
            .map(|v| v.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        crate::output::native_command("scp", &args, false)?;
        eprintln!(
            "Transfer may overwrite destination files. Destination directory must already exist."
        );
        Ok(())
    }

    pub fn execute(&self) -> Result<i32, String> {
        self.execute_program(OsStr::new("scp"))
    }

    pub fn execute_program(&self, program: &OsStr) -> Result<i32, String> {
        let status = Command::new(program)
            .args(&self.args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|_| {
                "Unable to start SCP; check that the OpenSSH scp client is installed in PATH"
            })?;
        if let Some(code) = status.code() {
            return Ok(code);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            Ok(status.signal().map(|s| 128 + s).unwrap_or(1))
        }
        #[cfg(not(unix))]
        {
            Ok(1)
        }
    }
}
