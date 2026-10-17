use chrono::Utc;
use clisshman::connect::{PingService, SshConnection};
use clisshman::model::Session;
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
    };

    let passthrough = vec!["-L".to_string(), "8080:localhost:80".to_string()];
    let args = SshConnection::build_args(&session, &passthrough);

    assert_eq!(
        args,
        vec![
            "-p",
            "2222",
            "-i",
            "~/.ssh/id_ed25519",
            "-J",
            "jump.example.com",
            "admin@bastion.example.com",
            "-L",
            "8080:localhost:80"
        ]
    );
}

#[test]
fn test_ping_tcp_unreachable() {
    let res = PingService::ping_tcp("127.0.0.1", 59999, Duration::from_millis(150));
    assert_eq!(res.ping_type, "tcp");
    assert!(!res.success);
}
