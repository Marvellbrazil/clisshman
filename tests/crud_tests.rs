use clisshman::{cli::*, service::AppService, storage::StorageManager};
use tempfile::tempdir;

fn add(service: &AppService, name: &str) -> clisshman::model::Session {
    service
        .add(AddArgs {
            name: Some(name.into()),
            target: Some("root@127.0.0.1:2222".into()),
            tags: vec!["work, Debian, web, debian".into()],
            ..Default::default()
        })
        .unwrap()
}

#[test]
fn lifecycle_duplicate_and_metadata() {
    let dir = tempdir().unwrap();
    let service = AppService::new(StorageManager::with_dir(dir.path().into()).unwrap());
    let original = add(&service, "alpha");
    assert_eq!(original.tags, vec!["Debian", "web", "work"]);
    service.favorite(&original.id, None, true).unwrap();
    service.record_attempt(&original.id).unwrap();
    let source = service.get(&original.id, None).unwrap();
    let copy = service
        .duplicate(DuplicateArgs {
            target: original.id.clone(),
            name: "beta".into(),
            ..Default::default()
        })
        .unwrap();
    assert_ne!(copy.id, original.id);
    assert!(!copy.favorite);
    assert!(copy.last_connect_attempt_at.is_none());
    assert_eq!(source, service.get(&original.id, None).unwrap());
    assert!(service
        .duplicate(DuplicateArgs {
            target: original.id.clone(),
            name: "alpha".into(),
            ..Default::default()
        })
        .is_err());
    let edited = service
        .edit(EditArgs {
            target: copy.id.clone(),
            host: Some("::1".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(edited.name, "beta");
    assert_eq!(edited.host, "::1");
    service.delete(&copy.id, None).unwrap();
    assert_eq!(add(&service, "gamma").id, "3");
}

#[test]
fn config_validation_and_hybrid_ids() {
    let dir = tempdir().unwrap();
    let service = AppService::new(StorageManager::with_dir(dir.path().into()).unwrap());
    assert!(service.set_config("default_port", "0").is_err());
    assert!(service.set_config("ping_timeout_seconds", "0").is_err());
    assert_eq!(add(&service, "integer").id, "1");
    service.set_config("id_type", "uuid").unwrap();
    let uuid = add(&service, "random");
    assert!(uuid::Uuid::parse_str(&uuid.id).is_ok());
    assert_eq!(service.get(&uuid.id[..8], None).unwrap().id, uuid.id);
    service.set_config("id_type", "integer").unwrap();
    assert_eq!(add(&service, "second").id, "2");
    service.set_config("confirm_delete", "0").unwrap();
    assert!(!service.storage.load_config().unwrap().confirm_delete);
}

#[test]
fn group_move_validates_final_uniqueness() {
    let dir = tempdir().unwrap();
    let service = AppService::new(StorageManager::with_dir(dir.path().into()).unwrap());
    let a = add(&service, "web");
    let b = service
        .add(AddArgs {
            name: Some("web".into()),
            group: Some("prod".into()),
            host: Some("localhost".into()),
            username: Some("deploy".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(service.get("web", None).is_err());
    assert!(service
        .edit(EditArgs {
            target: a.id.clone(),
            group: Some("prod".into()),
            ..Default::default()
        })
        .is_err());
    assert!(service.get(&a.id, None).unwrap().group.is_none());
    assert_eq!(service.get("web", Some("prod")).unwrap().id, b.id);
}

#[test]
fn validation_and_fuzzy_are_shared() {
    assert_eq!(
        clisshman::validation::parse_target("deploy@[::1]:2222").unwrap(),
        (Some("deploy".into()), "::1".into(), Some(2222))
    );
    assert!(clisshman::validation::parse_target("x@host:0").is_err());
    assert!(clisshman::selection::fuzzy_score("production-web", "pwb").is_some());
    assert!(clisshman::selection::fuzzy_score("web", "xyz").is_none());
    let dir = tempdir().unwrap();
    let service = AppService::new(StorageManager::with_dir(dir.path().into()).unwrap());
    let a = add(&service, "alpha");
    let b = add(&service, "beta");
    service.favorite(&b.id, None, true).unwrap();
    let ranked = clisshman::selection::ranked(service.list(&ListArgs::default()).unwrap(), "");
    assert_eq!(ranked[0].id, b.id);
    let ranked = clisshman::selection::ranked(ranked, "alpha");
    assert_eq!(ranked[0].id, a.id);
}
