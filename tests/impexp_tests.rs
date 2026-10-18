use chrono::Utc;
use clisshman::config::IdType;
use clisshman::impexp::SshConfigManager;
use clisshman::model::{Registry, Session};
use tempfile::tempdir;

fn session(id: &str, group: Option<&str>, name: &str, identity: Option<&str>) -> Session {
    Session {
        id: id.into(),
        group: group.map(Into::into),
        name: name.into(),
        host: "node1.internal".into(),
        user: "admin".into(),
        port: 2202,
        identity_file: identity.map(Into::into),
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
fn export_and_import_round_trip() {
    let mut reg = Registry {
        next_id: 2,
        ..Default::default()
    };
    reg.sessions.push(session(
        "1",
        Some("servers"),
        "test-node",
        Some("C:\\Users\\Me\\My Keys\\id_ed25519"),
    ));
    let dir = tempdir().unwrap();
    let path = dir.path().join("exported_ssh_config");
    SshConfigManager::export_to_writer(&reg, std::fs::File::create(&path).unwrap()).unwrap();
    let mut new_reg = Registry::default();
    let count = SshConfigManager::import_from_file(&path, &mut new_reg, IdType::Integer).unwrap();
    assert_eq!(count, 1);
    let imported = &new_reg.sessions[0];
    assert_eq!(imported.id, "1");
    assert_eq!(imported.name, "test-node");
    assert_eq!(imported.port, 2202);
    assert_eq!(
        imported.identity_file.as_deref(),
        Some("C:\\Users\\Me\\My Keys\\id_ed25519")
    );
}

#[test]
fn parse_supports_quotes_equals_and_multiple_aliases() {
    let parsed = SshConfigManager::parse(
        "# comment\nHost web api\n  HostName=10.0.0.5\n  User \"deploy\"\n  IdentityFile \"~/.ssh/my key\"\n  ProxyJump bastion.example.com\n",
    )
    .unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[1].name, "api");
    assert_eq!(parsed[0].host, "10.0.0.5");
    assert_eq!(parsed[0].identity_file.as_deref(), Some("~/.ssh/my key"));
    assert!(parsed.iter().all(|s| s.id.is_empty() && !s.favorite));
}

#[test]
fn parse_rejects_unrepresentable_grammar() {
    for content in [
        "Host *\n User deploy\n",
        "Include other\n",
        "Host a\n User x\nMatch host a\n",
        "User deploy\nHost a\n",
        "Host a\n HostName b\n",
        "Host a\n User x\n User y\n",
        "Host a\n User x\n ForwardAgent yes\n",
        "Host a\n User x\nHost a\n User y\n",
        "Host a\n User %u\n",
        "Host a\n User \"x\n",
    ] {
        assert!(SshConfigManager::parse(content).is_err(), "{content}");
    }
}

#[test]
fn export_generates_unique_safe_aliases() {
    let mut reg = Registry {
        next_id: 4,
        ..Default::default()
    };
    reg.sessions
        .push(session("1", Some("prod"), "web server", None));
    reg.sessions
        .push(session("2", Some("staging"), "web server", None));
    reg.sessions.push(session("3", None, "db", None));
    let mut out = Vec::new();
    SshConfigManager::export_to_writer(&reg, &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();
    let hosts: Vec<_> = text.lines().filter(|l| l.starts_with("Host ")).collect();
    assert_eq!(
        hosts,
        ["Host prod-web-server", "Host staging-web-server", "Host db"]
    );
    reg.sessions[2].host = "bad\nHost evil".into();
    assert!(SshConfigManager::export_to_writer(&reg, Vec::new()).is_err());
}
