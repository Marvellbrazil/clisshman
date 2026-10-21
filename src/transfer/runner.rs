use super::TransferPlan;
use std::ffi::OsStr;
use std::process::{Command, Stdio};
impl TransferPlan {
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
