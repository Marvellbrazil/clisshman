mod paths;
mod runner;
use crate::{cli::ScpArgs, model::Session};
use paths::{expand_home, local_operand, local_path, remote_operand};
use std::ffi::OsString;
use std::path::Path;
#[derive(Debug)]
pub struct TransferPlan {
    pub args: Vec<OsString>,
    pub download: bool,
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
}
