use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::model::Session;

pub struct SshConnection;

impl SshConnection {
    pub fn build_args(session: &Session, passthrough: &[String]) -> Vec<String> {
        let mut args = Vec::new();
        args.push("-p".to_string());
        args.push(session.port.to_string());

        if let Some(ref identity) = session.identity_file {
            args.push("-i".to_string());
            args.push(identity.clone());
        }

        if let Some(ref jump) = session.jump_host {
            args.push("-J".to_string());
            args.push(jump.clone());
        }

        let destination = format!("{}@{}", session.user, session.host);
        args.push(destination);

        for extra in passthrough {
            args.push(extra.clone());
        }

        args
    }

    pub fn execute(session: &Session, passthrough: &[String]) -> Result<i32, String> {
        let args = Self::build_args(session, passthrough);

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let err = Command::new("ssh").args(&args).exec();
            return Err(format!("Failed to exec ssh: {err}"));
        }

        #[cfg(not(unix))]
        {
            let mut child = Command::new("ssh")
                .args(&args)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()
                .map_err(|e| format!("Failed to spawn native ssh: {e}"))?;

            let status = child
                .wait()
                .map_err(|e| format!("Error waiting on ssh process: {e}"))?;

            Ok(status.code().unwrap_or(0))
        }
    }
}

pub struct PingService;

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
        let addr_str = format!("{}:{}", host, port);
        let start = Instant::now();

        let resolved = match addr_str.to_socket_addrs() {
            Ok(addrs) => addrs.collect::<Vec<SocketAddr>>(),
            Err(e) => {
                return PingResult {
                    target: addr_str,
                    host: host.to_string(),
                    port: Some(port),
                    ping_type: "tcp".to_string(),
                    success: false,
                    latency_ms: None,
                    message: format!("DNS resolution failed: {e}"),
                };
            }
        };

        if resolved.is_empty() {
            return PingResult {
                target: addr_str,
                host: host.to_string(),
                port: Some(port),
                ping_type: "tcp".to_string(),
                success: false,
                latency_ms: None,
                message: "No socket address resolved".to_string(),
            };
        }

        let mut connected = false;
        let mut last_err = String::new();

        for addr in resolved {
            match TcpStream::connect_timeout(&addr, timeout) {
                Ok(_) => {
                    connected = true;
                    break;
                }
                Err(e) => {
                    last_err = e.to_string();
                }
            }
        }

        let duration = start.elapsed().as_millis();

        if connected {
            PingResult {
                target: addr_str,
                host: host.to_string(),
                port: Some(port),
                ping_type: "tcp".to_string(),
                success: true,
                latency_ms: Some(duration),
                message: format!("TCP connection established in {duration}ms"),
            }
        } else {
            PingResult {
                target: addr_str,
                host: host.to_string(),
                port: Some(port),
                ping_type: "tcp".to_string(),
                success: false,
                latency_ms: None,
                message: format!("TCP connection failed: {last_err}"),
            }
        }
    }

    pub fn ping_icmp(host: &str, timeout_secs: u64) -> PingResult {
        let start = Instant::now();

        let status = if cfg!(target_os = "windows") {
            let timeout_ms = (timeout_secs * 1000).to_string();
            Command::new("ping")
                .args(["-n", "1", "-w", &timeout_ms, host])
                .output()
        } else if cfg!(target_os = "macos") {
            let timeout_s = timeout_secs.to_string();
            Command::new("ping")
                .args(["-c", "1", "-W", &timeout_s, host])
                .output()
        } else {
            let timeout_s = timeout_secs.to_string();
            Command::new("ping")
                .args(["-c", "1", "-W", &timeout_s, host])
                .output()
        };

        let duration = start.elapsed().as_millis();

        match status {
            Ok(out) => {
                let success = out.status.success();
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let msg = if success {
                    format!("ICMP echo reply received in {duration}ms")
                } else if !stderr.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    stdout.lines().last().unwrap_or("Ping failed").to_string()
                };

                PingResult {
                    target: host.to_string(),
                    host: host.to_string(),
                    port: None,
                    ping_type: "icmp".to_string(),
                    success,
                    latency_ms: if success { Some(duration) } else { None },
                    message: msg,
                }
            }
            Err(e) => PingResult {
                target: host.to_string(),
                host: host.to_string(),
                port: None,
                ping_type: "icmp".to_string(),
                success: false,
                latency_ms: None,
                message: format!("Failed to spawn system ping: {e}"),
            },
        }
    }
}
