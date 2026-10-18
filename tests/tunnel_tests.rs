use clisshman::{
    cli::*,
    model::TunnelKind,
    service::{tunnel_args, AppService},
    storage::StorageManager,
};
#[test]
fn tunnel_crud_args_and_session_references() {
    let dir = tempfile::tempdir().unwrap();
    let service = AppService::new(StorageManager::with_dir(dir.path().into()).unwrap());
    let session = service
        .add(AddArgs {
            host: Some("localhost".into()),
            username: Some("deploy".into()),
            ..Default::default()
        })
        .unwrap();
    let tunnel = service
        .add_tunnel(TunnelAddArgs {
            name: "database".into(),
            session: session.id.clone(),
            group: None,
            kind: TunnelKind::Local,
            bind: "127.0.0.1".into(),
            listen_port: 15432,
            destination_host: Some("db.internal".into()),
            destination_port: Some(5432),
            allow_public_bind: false,
        })
        .unwrap();
    assert_eq!(
        tunnel_args(&tunnel).unwrap(),
        [
            "-N",
            "-o",
            "ExitOnForwardFailure=yes",
            "-L",
            "127.0.0.1:15432:db.internal:5432"
        ]
    );
    assert!(service.delete(&session.id, None).is_err());
    let mut remote = tunnel.clone();
    remote.kind = TunnelKind::Remote;
    remote.destination_host = Some("::1".into());
    assert_eq!(
        tunnel_args(&remote).unwrap().last().unwrap(),
        "127.0.0.1:15432:[::1]:5432"
    );
    assert_eq!(tunnel_args(&remote).unwrap()[3], "-R");
    remote.destination_port = Some(0);
    assert!(tunnel_args(&remote).is_err());
    let mut dynamic_invalid = tunnel.clone();
    dynamic_invalid.kind = TunnelKind::Dynamic;
    assert!(tunnel_args(&dynamic_invalid).is_err());
    let dynamic = service
        .edit_tunnel(TunnelEditArgs {
            target: tunnel.id.clone(),
            kind: Some(TunnelKind::Dynamic),
            bind: Some("::1".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(dynamic.destination_host.is_none());
    assert_eq!(
        tunnel_args(&dynamic).unwrap().last().unwrap(),
        "[::1]:15432"
    );
    assert!(service
        .edit_tunnel(TunnelEditArgs {
            target: tunnel.id.clone(),
            bind: Some("0.0.0.0".into()),
            ..Default::default()
        })
        .is_err());
    service
        .edit_tunnel(TunnelEditArgs {
            target: tunnel.id.clone(),
            bind: Some("0.0.0.0".into()),
            allow_public_bind: true,
            ..Default::default()
        })
        .unwrap();
    service.delete_tunnel(&tunnel.id).unwrap();
    service.delete(&session.id, None).unwrap();
}
