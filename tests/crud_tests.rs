use tempfile::tempdir;
use clisshman::cli::*;
use clisshman::service::AppService;
use clisshman::storage::StorageManager;

#[test]
fn test_crud_full_lifecycle() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().to_path_buf()).unwrap();
    let service = AppService::new(storage.clone());

    let add_res = service.handle_add(
        AddArgs {
            target: Some("root@192.168.1.50:2222".to_string()),
            name: Some("server-alpha".to_string()),
            group: Some("production".to_string()),
            host: None,
            username: None,
            port: None,
            identity_file: Some("~/.ssh/id_rsa".to_string()),
            jump_host: None,
            description: Some("Alpha testing server".to_string()),
            tags: vec!["web,debian,infra".to_string()],
        },
        true,
    );
    assert!(add_res.is_ok());

    let reg = storage.load_registry().unwrap();
    assert_eq!(reg.sessions.len(), 1);
    let s = &reg.sessions[0];
    assert_eq!(s.id, "1");
    assert_eq!(s.name, "server-alpha");
    assert_eq!(s.user, "root");
    assert_eq!(s.host, "192.168.1.50");
    assert_eq!(s.port, 2222);
    assert_eq!(s.tags, vec!["debian", "infra", "web"]);

    let edit_res = service.handle_edit(
        EditArgs {
            target: "1".to_string(),
            name: Some("server-alpha-v2".to_string()),
            group: None,
            host: None,
            username: None,
            port: Some(2200),
            identity_file: None,
            jump_host: None,
            description: None,
            tags: vec![],
            clear_tags: false,
        },
        true,
    );
    assert!(edit_res.is_ok());

    let reg = storage.load_registry().unwrap();
    assert_eq!(reg.sessions[0].name, "server-alpha-v2");
    assert_eq!(reg.sessions[0].port, 2200);

    let del_res = service.handle_delete(
        DeleteArgs {
            target: "server-alpha-v2".to_string(),
            group: Some("production".to_string()),
            yes: true,
        },
        true,
    );
    assert!(del_res.is_ok());

    let reg = storage.load_registry().unwrap();
    assert_eq!(reg.sessions.len(), 0);
}

#[test]
fn test_config_get_set() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().to_path_buf()).unwrap();
    let service = AppService::new(storage.clone());

    service
        .handle_config(
            ConfigArgs {
                key: Some("confirm_delete".to_string()),
                value: Some("0".to_string()),
            },
            true,
        )
        .unwrap();

    let cfg = storage.load_config().unwrap();
    assert!(!cfg.confirm_delete);

    service
        .handle_config(
            ConfigArgs {
                key: Some("id_type".to_string()),
                value: Some("uuid".to_string()),
            },
            true,
        )
        .unwrap();

    let cfg = storage.load_config().unwrap();
    assert_eq!(cfg.id_type, clisshman::config::IdType::Uuid);
}
