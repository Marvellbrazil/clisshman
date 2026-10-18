use clisshman::{
    cli::{AddArgs, ListArgs},
    service::AppService,
    storage::StorageManager,
};
#[test]
fn uuid_prefix_ambiguity_and_numeric_id_priority() {
    let dir = tempfile::tempdir().unwrap();
    let service = AppService::new(StorageManager::with_dir(dir.path().into()).unwrap());
    for name in ["first", "second"] {
        service
            .add(AddArgs {
                name: Some(name.into()),
                host: Some("localhost".into()),
                username: Some("deploy".into()),
                ..Default::default()
            })
            .unwrap();
    }
    service
        .storage
        .update_registry(|registry| {
            registry.sessions[0].id = "abcdef12-1111-4111-8111-111111111111".into();
            registry.sessions[1].id = "abcdef12-2222-4222-8222-222222222222".into();
            Ok(())
        })
        .unwrap();
    assert!(service.get("abcdef12", None).is_err());
    assert!(service.get("abcdef", None).is_err());
    assert_eq!(
        service
            .get("abcdef12-1111-4111-8111-111111111111", None)
            .unwrap()
            .name,
        "first"
    );
}
#[test]
fn list_uses_numeric_order_and_all_tags() {
    let dir = tempfile::tempdir().unwrap();
    let service = AppService::new(StorageManager::with_dir(dir.path().into()).unwrap());
    for n in 0..12 {
        service
            .add(AddArgs {
                name: Some(format!("server-{n}")),
                host: Some("localhost".into()),
                username: Some("deploy".into()),
                tags: vec!["work,Debian".into()],
                ..Default::default()
            })
            .unwrap();
    }
    let list = service
        .list(&ListArgs {
            tags: vec!["WORK,debian".into()],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(list[9].id, "10");
    assert_eq!(list.len(), 12);
    assert!(service
        .list(&ListArgs {
            tags: vec!["work,missing".into()],
            ..Default::default()
        })
        .unwrap()
        .is_empty());
}
