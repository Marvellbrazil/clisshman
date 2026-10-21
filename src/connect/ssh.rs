use crate::model::Session;
use directories::BaseDirs;
use std::ffi::OsString;
use std::process::{Command, ExitStatus, Stdio};
pub struct SshConnection;

impl SshConnection {
    pub fn build_args(session: &Session, passthrough: &[String]) -> Vec<String> {
        Self::native_args(session, passthrough)
            .into_iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    fn native_args(session: &Session, passthrough: &[String]) -> Vec<OsString> {
        let mut args = vec![
            OsString::from("-p"),
            OsString::from(session.port.to_string()),
        ];

        if let Some(identity) = &session.identity_file {
            let expanded = identity
                .strip_prefix('~')
                .filter(|suffix| suffix.starts_with('/'))
                .and_then(|suffix| {
                    BaseDirs::new().map(|dirs| {
                        let mut path = dirs.home_dir().as_os_str().to_os_string();
                        path.push(suffix);
                        path
                    })
                })
                .unwrap_or_else(|| OsString::from(identity));
            args.push(OsString::from("-i"));
            args.push(expanded);
        }

        if let Some(jump) = &session.jump_host {
            args.push(OsString::from("-J"));
            args.push(OsString::from(jump));
        }

        args.extend(passthrough.iter().map(OsString::from));
        args.push(OsString::from(format!("{}@{}", session.user, session.host)));
        args
    }

    pub fn execute(session: &Session, passthrough: &[String]) -> Result<i32, String> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let error = Command::new("ssh")
                .args(Self::native_args(session, passthrough))
                .exec();
            Err(if error.kind() == std::io::ErrorKind::NotFound {
                "Unable to start SSH: ssh was not found in PATH".to_string()
            } else {
                "Unable to start SSH".to_string()
            })
        }

        #[cfg(not(unix))]
        {
            Self::execute_returning(session, passthrough)
        }
    }

    pub fn execute_returning(session: &Session, passthrough: &[String]) -> Result<i32, String> {
        let status = Command::new("ssh")
            .args(Self::native_args(session, passthrough))
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|_| "Unable to start SSH".to_string())?
            .wait()
            .map_err(|_| "Unable to wait for SSH".to_string())?;

        Ok(Self::exit_code(status))
    }

    fn exit_code(status: ExitStatus) -> i32 {
        if let Some(code) = status.code() {
            return code;
        }

        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            status.signal().map(|signal| 128 + signal).unwrap_or(1)
        }

        #[cfg(not(unix))]
        {
            1
        }
    }
}
