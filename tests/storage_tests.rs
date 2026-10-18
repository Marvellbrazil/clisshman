use clisshman::{
    cli::AddArgs, model::normalize_tags, service::AppService, storage::StorageManager,
};
use tempfile::tempdir;

#[test]
fn tags_trim_deduplicate_and_sort_case_insensitively() {
    assert_eq!(
        normalize_tags(&["work, Debian, Web Server, debian, api,,".into()]),
        vec!["api", "Debian", "Web Server", "work"]
    );
}
#[test]
fn migration_backs_up_original_and_preserves_ids() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().into()).unwrap();
    let original = br#"{"version":1,"next_id":1,"sessions":[]}"#;
    std::fs::write(storage.registry_path(), original).unwrap();
    assert_eq!(storage.load_registry().unwrap().version, 1);
    storage.update_registry(|_| Ok(())).unwrap();
    assert_eq!(storage.load_registry().unwrap().version, 2);
    storage.update_registry(|_| Ok(())).unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("sessions.v1.backup.json")).unwrap(),
        original
    );
}
#[test]
fn transaction_rolls_back_and_future_schema_is_rejected() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().into()).unwrap();
    storage.update_registry(|_| Ok(())).unwrap();
    let before = std::fs::read(storage.registry_path()).unwrap();
    assert!(storage
        .update_registry(|registry| {
            registry.next_id = 100;
            Err::<(), _>("stop".into())
        })
        .is_err());
    assert_eq!(std::fs::read(storage.registry_path()).unwrap(), before);
    std::fs::write(
        storage.registry_path(),
        br#"{"version":999,"next_id":1,"sessions":[]}"#,
    )
    .unwrap();
    assert!(storage.load_registry().is_err());
}
#[test]
fn migration_preserves_legacy_session_metadata() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().into()).unwrap();
    let original = br#"{"version":1,"next_id":2,"sessions":[{"id":"1","name":"legacy","host":"localhost","user":"deploy","port":22,"tags":["work","Debian","debian"],"created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"}]}"#;
    std::fs::write(storage.registry_path(), original).unwrap();
    storage.update_registry(|_| Ok(())).unwrap();
    let registry = storage.load_registry().unwrap();
    assert_eq!(registry.sessions[0].id, "1");
    assert!(!registry.sessions[0].favorite);
    assert!(registry.sessions[0].last_connect_attempt_at.is_none());
    assert_eq!(registry.sessions[0].tags, vec!["Debian", "work"]);
    assert_eq!(
        std::fs::read(dir.path().join("sessions.v1.backup.json")).unwrap(),
        original
    );
}
#[test]
fn independent_services_do_not_lose_updates() {
    let dir = tempdir().unwrap();
    let mut threads = vec![];
    for n in 0..16 {
        let path = dir.path().to_path_buf();
        threads.push(std::thread::spawn(move || {
            let service = AppService::new(StorageManager::with_dir(path).unwrap());
            service
                .add(AddArgs {
                    name: Some(format!("server-{n}")),
                    host: Some("localhost".into()),
                    username: Some("deploy".into()),
                    ..Default::default()
                })
                .unwrap();
        }));
    }
    for thread in threads {
        thread.join().unwrap();
    }
    let registry = StorageManager::with_dir(dir.path().into())
        .unwrap()
        .load_registry()
        .unwrap();
    assert_eq!(registry.sessions.len(), 16);
    assert_eq!(registry.next_id, 17);
    assert_eq!(
        registry
            .sessions
            .iter()
            .map(|s| s.id.clone())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        16
    );
}
