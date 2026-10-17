use chrono::Utc;
use tempfile::tempdir;
use clisshman::config::IdType;
use clisshman::impexp::SshConfigManager;
use clisshman::model::{Registry, Session};

#[test]
fn test_export_and_import_ssh_config() {
    let mut reg = Registry::default();
    reg.sessions.push(Session {
        id: "1".to_string(),
        group: Some("servers".to_string()),
        name: "test-node".to_string(),
        host: "node1.internal".to_string(),
        user: "admin".to_string(),
        port: 2202,
        identity_file: Some("~/.ssh/custom_key".to_string()),
        jump_host: None,
        description: None,
        tags: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

    let dir = tempdir().unwrap();
    let export_path = dir.path().join("exported_ssh_config");

    let file = std::fs::File::create(&export_path).unwrap();
    SshConfigManager::export_to_writer(&reg, file).unwrap();

    let mut new_reg = Registry::default();
    let count = SshConfigManager::import_from_file(&export_path, &mut new_reg, IdType::Integer).unwrap();

    assert_eq!(count, 1);
    assert_eq!(new_reg.sessions.len(), 1);
    let imported = &new_reg.sessions[0];
    assert_eq!(imported.name, "test-node");
    assert_eq!(imported.host, "node1.internal");
    assert_eq!(imported.user, "admin");
    assert_eq!(imported.port, 2202);
    assert_eq!(imported.identity_file, Some("~/.ssh/custom_key".to_string()));
}
