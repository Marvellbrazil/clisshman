use chrono::Utc;
use clisshman::connect::{IcmpPlatform, PingService, SshConnection};
use clisshman::model::Session;
use std::net::TcpListener;
use std::time::Duration;

#[test]
fn test_build_ssh_args() {
    let session = Session {
        id: "1".to_string(),
        group: Some("prod".to_string()),
        name: "bastion".to_string(),
        host: "bastion.example.com".to_string(),
        user: "admin".to_string(),
        port: 2222,
        identity_file: Some("~/.ssh/id_ed25519".to_string()),
        jump_host: Some("jump.example.com".to_string()),
        description: None,
        tags: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        favorite: false,
        last_connect_attempt_at: None,
    };

    let passthrough = vec!["-L".to_string(), "8080:localhost:80".to_string()];
    let args = SshConnection::build_args(&session, &passthrough);

    let mut identity = directories::BaseDirs::new()
        .expect("home directory")
        .home_dir()
        .as_os_str()
        .to_os_string();
    identity.push("/.ssh/id_ed25519");
    assert_eq!(
        args,
        vec![
            "-p",
            "2222",
            "-i",
            &identity.to_string_lossy(),
            "-J",
            "jump.example.com",
            "-L",
            "8080:localhost:80",
            "admin@bastion.example.com",
        ]
    );
}

fn session() -> Session {
    Session {
        id: "1".to_string(),
        group: None,
        name: "local".to_string(),
        host: "127.0.0.1".to_string(),
        user: "admin".to_string(),
        port: 22,
        identity_file: None,
        jump_host: None,
        description: None,
        tags: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        favorite: false,
        last_connect_attempt_at: None,
    }
}

#[test]
fn test_build_tunnel_args_before_destination() {
    let passthrough = vec![
        "-N".to_string(),
        "-o".to_string(),
        "ExitOnForwardFailure=yes".to_string(),
        "-L".to_string(),
        "8080:localhost:80".to_string(),
        "-R".to_string(),
        "9090:localhost:90".to_string(),
        "-D".to_string(),
        "1080".to_string(),
    ];
    let args = SshConnection::build_args(&session(), &passthrough);
    assert_eq!(&args[2..args.len() - 1], &passthrough);
    assert_eq!(args.last().map(String::as_str), Some("admin@127.0.0.1"));
}

#[test]
fn test_identity_and_passthrough_remain_literal() {
    let mut session = session();
    let identity = "~other/key $(printf unsafe); spaces";
    session.identity_file = Some(identity.to_string());
    let passthrough = vec!["-o".to_string(), "ProxyCommand=literal ; value".to_string()];
    let args = SshConnection::build_args(&session, &passthrough);
    assert_eq!(args[3], identity);
    assert_eq!(args[5], passthrough[1]);
}

#[test]
fn test_home_identity_suffix_remains_literal() {
    let mut session = session();
    session.identity_file = Some("~//.ssh/key with spaces;$(literal)".to_string());
    let args = SshConnection::build_args(&session, &[]);
    let home = directories::BaseDirs::new().expect("home directory");
    let mut expected = home.home_dir().as_os_str().to_os_string();
    expected.push("//.ssh/key with spaces;$(literal)");
    assert_eq!(args[3], expected.to_string_lossy());
}

#[test]
fn test_ping_tcp_local_listener() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind local listener");
    let port = listener.local_addr().expect("listener address").port();
    let result = PingService::ping_tcp("127.0.0.1", port, Duration::from_secs(1));
    assert!(result.success, "{}", result.message);
    assert_eq!(result.target, format!("127.0.0.1:{port}"));
    assert_eq!(result.port, Some(port));
    assert_eq!(result.ping_type, "tcp");
    assert!(result.latency_ms.is_some());
}

#[test]
fn test_ping_tcp_ipv6_listener() {
    let listener = match TcpListener::bind(("::1", 0)) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("IPv6 loopback unavailable: {}", error.kind());
            return;
        }
    };
    let port = listener.local_addr().expect("listener address").port();
    let result = PingService::ping_tcp("::1", port, Duration::from_secs(1));
    assert!(result.success, "{}", result.message);
    assert_eq!(result.target, format!("[::1]:{port}"));
}

#[test]
fn test_ping_tcp_localhost_resolution() {
    let listener = TcpListener::bind(("localhost", 0)).expect("bind local listener");
    let port = listener.local_addr().expect("listener address").port();
    let result = PingService::ping_tcp("localhost", port, Duration::from_secs(1));
    assert!(result.success, "{}", result.message);
}

#[test]
fn test_ping_tcp_unreachable() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("reserve local port");
    let port = listener.local_addr().expect("listener address").port();
    drop(listener);
    let result = PingService::ping_tcp("127.0.0.1", port, Duration::from_millis(150));
    assert_eq!(result.ping_type, "tcp");
    assert!(!result.success);
    assert!(result.latency_ms.is_none());
    assert!(matches!(
        result.message.as_str(),
        "TCP connection failed" | "TCP connection timed out"
    ));
}

#[test]
fn test_ping_tcp_invalid_timeout() {
    for timeout in [Duration::ZERO, Duration::MAX] {
        let result = PingService::ping_tcp("invalid host", 22, timeout);
        assert!(!result.success);
        assert_eq!(result.message, "Invalid timeout");
        assert!(result.latency_ms.is_none());
    }
}

#[test]
fn test_icmp_platform_arguments() {
    for (platform, count, flag, timeout) in [
        (IcmpPlatform::Windows, "-n", "-w", "2000"),
        (IcmpPlatform::MacOs, "-c", "-W", "2000"),
        (IcmpPlatform::Linux, "-c", "-W", "2"),
    ] {
        assert_eq!(
            PingService::build_icmp_args("::1", 2, platform).expect("valid ping arguments"),
            vec![count, "1", flag, timeout, "::1"]
        );
    }
}

#[test]
fn test_icmp_timeout_validation() {
    for platform in [
        IcmpPlatform::Windows,
        IcmpPlatform::MacOs,
        IcmpPlatform::Linux,
    ] {
        for timeout in [0, u64::MAX] {
            assert_eq!(
                PingService::build_icmp_args("127.0.0.1", timeout, platform),
                Err("Invalid timeout".to_string())
            );
        }
    }
    for (platform, limit) in [
        (IcmpPlatform::Windows, u64::from(u32::MAX) / 1000),
        (IcmpPlatform::MacOs, i32::MAX as u64 / 1000),
        (IcmpPlatform::Linux, i32::MAX as u64),
    ] {
        assert!(PingService::build_icmp_args("127.0.0.1", limit, platform).is_ok());
        assert!(PingService::build_icmp_args("127.0.0.1", limit + 1, platform).is_err());
    }
    let result = PingService::ping_icmp("127.0.0.1", 0);
    assert!(!result.success);
    assert_eq!(result.message, "Invalid timeout");
}

#[test]
fn test_icmp_host_validation() {
    for host in ["", "-f", "bad\0host"] {
        assert_eq!(
            PingService::build_icmp_args(host, 1, IcmpPlatform::Linux),
            Err("Invalid host".to_string())
        );
    }
}

#[test]
fn test_returning_runner_reports_spawn_error() {
    let mut session = session();
    session.identity_file = Some("invalid\0path".to_string());
    assert_eq!(
        SshConnection::execute_returning(&session, &[]),
        Err("Unable to start SSH".to_string())
    );
}

#[cfg(unix)]
#[test]
fn test_returning_runner_probe() {
    let expected = match std::env::var("CLISSHMAN_TEST_SSH_EXIT") {
        Ok(expected) => expected.parse::<i32>().expect("expected exit code"),
        Err(_) => return,
    };
    assert_eq!(
        SshConnection::execute_returning(&session(), &[]),
        Ok(expected)
    );
    if let Ok(marker) = std::env::var("CLISSHMAN_TEST_SSH_MARKER") {
        assert!(std::path::Path::new(&marker).is_file());
    }
}

#[cfg(unix)]
#[test]
fn test_returning_runner_waits_and_preserves_exit_status() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let directory = tempfile::tempdir().expect("SSH fixture directory");
    let ssh = directory.path().join("ssh");
    let marker = directory.path().join("finished");
    for (script, expected) in [
        (
            "#!/bin/sh\n: > \"$CLISSHMAN_TEST_SSH_MARKER\"\nexit 0\n",
            "0",
        ),
        ("#!/bin/sh\nexit 23\n", "23"),
        ("#!/bin/sh\nkill -TERM $$\n", "143"),
    ] {
        std::fs::write(&ssh, script).expect("write SSH fixture");
        std::fs::set_permissions(&ssh, std::fs::Permissions::from_mode(0o700))
            .expect("SSH fixture permissions");
        let output = Command::new(std::env::current_exe().expect("current test executable"))
            .args(["--exact", "test_returning_runner_probe", "--nocapture"])
            .env("PATH", directory.path())
            .env("CLISSHMAN_TEST_SSH_EXIT", expected)
            .env("CLISSHMAN_TEST_SSH_MARKER", &marker)
            .output()
            .expect("run isolated SSH probe");
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
