use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::process::Command;
use std::time::{Duration, Instant};
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
