use directories::BaseDirs;
use std::ffi::OsString;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use crate::model::Session;

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
            Command::new("ssh")
                .args(Self::native_args(session, passthrough))
                .exec();
            Err("Unable to start SSH".to_string())
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

pub struct PingService;

#[derive(Debug, Clone, Copy)]
pub enum IcmpPlatform {
    Windows,
    MacOs,
    Linux,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PingResult {
    pub target: String,
    pub host: String,
    pub port: Option<u16>,
    pub ping_type: String,
    pub success: bool,
    pub latency_ms: Option<u128>,
    pub message: String,
}

impl PingService {
    pub fn ping_tcp(host: &str, port: u16, timeout: Duration) -> PingResult {
        let target = if host.contains(':') {
            format!("[{host}]:{port}")
        } else {
            format!("{host}:{port}")
        };
        let mut result = PingResult {
            target,
            host: host.to_string(),
            port: Some(port),
            ping_type: "tcp".to_string(),
            success: false,
            latency_ms: None,
            message: "TCP connection failed".to_string(),
        };
        let start = Instant::now();
        if timeout.is_zero() || start.checked_add(timeout).is_none() {
            result.message = "Invalid timeout".to_string();
            return result;
        }

        let resolved = match (host, port).to_socket_addrs() {
            Ok(addrs) => addrs.collect::<Vec<SocketAddr>>(),
            Err(_) => {
                result.message = "DNS resolution failed".to_string();
                return result;
            }
        };

        if resolved.is_empty() {
            result.message = "No socket address resolved".to_string();
            return result;
        }

        for addr in resolved {
            let remaining = match timeout.checked_sub(start.elapsed()) {
                Some(remaining) if !remaining.is_zero() => remaining,
                _ => {
                    result.message = "TCP connection timed out".to_string();
                    break;
                }
            };
            match TcpStream::connect_timeout(&addr, remaining) {
                Ok(_) => {
                    let duration = start.elapsed().as_millis();
                    result.success = true;
                    result.latency_ms = Some(duration);
                    result.message = format!("TCP connection established in {duration}ms");
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                    result.message = "TCP connection timed out".to_string();
                }
                Err(_) => {}
            }
        }

        result
    }

    pub fn build_icmp_args(
        host: &str,
        timeout_secs: u64,
        platform: IcmpPlatform,
    ) -> Result<Vec<String>, String> {
        if timeout_secs == 0 {
            return Err("Invalid timeout".to_string());
        }
        if host.is_empty() || host.starts_with('-') || host.contains('\0') {
            return Err("Invalid host".to_string());
        }

        let (count_flag, timeout_flag, timeout) = match platform {
            IcmpPlatform::Windows => {
                let timeout_ms = timeout_secs
                    .checked_mul(1000)
                    .and_then(|millis| u32::try_from(millis).ok())
                    .ok_or_else(|| "Invalid timeout".to_string())?;
                ("-n", "-w", timeout_ms.to_string())
            }
            IcmpPlatform::MacOs => {
                let timeout_ms = timeout_secs
                    .checked_mul(1000)
                    .and_then(|millis| i32::try_from(millis).ok())
                    .ok_or_else(|| "Invalid timeout".to_string())?;
                ("-c", "-W", timeout_ms.to_string())
            }
            IcmpPlatform::Linux => {
                let timeout_secs =
                    i32::try_from(timeout_secs).map_err(|_| "Invalid timeout".to_string())?;
                ("-c", "-W", timeout_secs.to_string())
            }
        };

        Ok(vec![
            count_flag.to_string(),
            "1".to_string(),
            timeout_flag.to_string(),
            timeout,
            host.to_string(),
        ])
    }

    pub fn ping_icmp(host: &str, timeout_secs: u64) -> PingResult {
        let mut result = PingResult {
            target: host.to_string(),
            host: host.to_string(),
            port: None,
            ping_type: "icmp".to_string(),
            success: false,
            latency_ms: None,
            message: "ICMP echo failed".to_string(),
        };
        let platform = if cfg!(target_os = "windows") {
            IcmpPlatform::Windows
        } else if cfg!(target_os = "macos") {
            IcmpPlatform::MacOs
        } else {
            IcmpPlatform::Linux
        };
        let args = match Self::build_icmp_args(host, timeout_secs, platform) {
            Ok(args) => args,
            Err(message) => {
                result.message = message;
                return result;
            }
        };
        let start = Instant::now();

        match Command::new("ping").args(args).output() {
            Ok(output) if output.status.success() => {
                let duration = start.elapsed().as_millis();
                result.success = true;
                result.latency_ms = Some(duration);
                result.message = format!("ICMP echo reply received in {duration}ms");
            }
            Ok(_) => {}
            Err(_) => result.message = "Unable to start system ping".to_string(),
        }

        result
    }
}
