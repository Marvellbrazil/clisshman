use std::process::{Command, Output, Stdio};
#[test]
fn clap_command_tree_is_consistent() {
    use clap::CommandFactory;
    clisshman::cli::Cli::command().debug_assert();
}
fn run(dir: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clisshman"))
        .args(args)
        .env("CLISSHMAN_CONFIG_DIR", dir)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}
#[test]
fn noninteractive_confirmation_and_json_contracts() {
    let dir = tempfile::tempdir().unwrap();
    let added = run(
        dir.path(),
        &[
            "add",
            "-H",
            "localhost",
            "-u",
            "deploy",
            "-n",
            "alpha",
            "--json",
        ],
    );
    assert!(
        added.status.success(),
        "{}",
        String::from_utf8_lossy(&added.stderr)
    );
    let s: serde_json::Value = serde_json::from_slice(&added.stdout).unwrap();
    assert_eq!(s["id"], "1");
    assert!(!run(dir.path(), &["delete", "1"]).status.success());
    assert!(run(dir.path(), &["config", "confirm_delete", "0"])
        .status
        .success());
    assert!(!run(dir.path(), &["delete", "1"]).status.success());
    let before = std::fs::read(dir.path().join("sessions.json")).unwrap();
    let dry = run(dir.path(), &["connect", "1", "--dry-run", "--json"]);
    assert!(dry.status.success());
    let _: serde_json::Value = serde_json::from_slice(&dry.stdout).unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("sessions.json")).unwrap(),
        before
    );
    assert!(run(dir.path(), &["delete", "1", "--yes"]).status.success());
}
#[test]
fn concurrent_process_adds_preserve_every_entry() {
    let dir = tempfile::tempdir().unwrap();
    let mut processes = vec![];
    for n in 0..8 {
        processes.push(
            Command::new(env!("CARGO_BIN_EXE_clisshman"))
                .args([
                    "add",
                    "-H",
                    "localhost",
                    "-u",
                    "deploy",
                    "-n",
                    &format!("node-{n}"),
                ])
                .env("CLISSHMAN_CONFIG_DIR", dir.path())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    for process in processes {
        let out = process.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let out = run(dir.path(), &["list", "--json"]);
    let sessions: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(sessions.len(), 8);
}
#[test]
fn import_preview_conflict_confirmation_and_json_backup() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("ssh_config");
    std::fs::write(&file, "Host alpha\n HostName localhost\n User deploy\n").unwrap();
    let file = file.to_str().unwrap();
    assert!(run(dir.path(), &["import", file, "--dry-run"])
        .status
        .success());
    assert!(!dir.path().join("sessions.json").exists());
    assert!(run(dir.path(), &["import", file]).status.success());
    assert!(!run(dir.path(), &["import", file]).status.success());
    assert!(run(dir.path(), &["import", file, "--yes"]).status.success());
    let backup = dir.path().join("backup.json");
    let backup = backup.to_str().unwrap();
    assert!(
        run(dir.path(), &["export", "--format", "json", "-o", backup])
            .status
            .success()
    );
    assert!(
        !run(dir.path(), &["export", "--format", "json", "-o", backup])
            .status
            .success()
    );
    let restore = tempfile::tempdir().unwrap();
    assert!(run(restore.path(), &["import", backup, "--format", "json"])
        .status
        .success());
}
#[test]
fn imported_overwrite_retains_id_favorite_and_tunnel_references() {
    let dir = tempfile::tempdir().unwrap();
    let service = clisshman::service::AppService::new(
        clisshman::storage::StorageManager::with_dir(dir.path().into()).unwrap(),
    );
    let session = service
        .add(clisshman::cli::AddArgs {
            name: Some("alpha".into()),
            group: Some("imported".into()),
            host: Some("old.internal".into()),
            username: Some("deploy".into()),
            ..Default::default()
        })
        .unwrap();
    service.favorite(&session.id, None, true).unwrap();
    service
        .add_tunnel(clisshman::cli::TunnelAddArgs {
            name: "socks".into(),
            session: session.id.clone(),
            group: None,
            kind: clisshman::model::TunnelKind::Dynamic,
            bind: "127.0.0.1".into(),
            listen_port: 1080,
            destination_host: None,
            destination_port: None,
            allow_public_bind: false,
        })
        .unwrap();
    let file = dir.path().join("ssh_config");
    std::fs::write(&file, "Host alpha\n HostName new.internal\n User deploy\n").unwrap();
    let out = run(dir.path(), &["import", file.to_str().unwrap(), "--yes"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let updated = service.get(&session.id, None).unwrap();
    assert_eq!(updated.host, "new.internal");
    assert!(updated.favorite);
    assert_eq!(updated.created_at, session.created_at);
    assert_eq!(service.get_tunnel("socks").unwrap().session_id, session.id);
}
#[test]
fn pipe_output_and_json_errors_are_clean() {
    let dir = tempfile::tempdir().unwrap();
    assert!(run(
        dir.path(),
        &["add", "-H", "localhost", "-u", "deploy", "-n", "alpha"]
    )
    .status
    .success());
    let list = run(dir.path(), &["list"]);
    assert!(list.status.success());
    assert!(!list.stdout.contains(&27));
    assert!(!String::from_utf8_lossy(&list.stdout).contains('│'));
    let failure = run(dir.path(), &["show", "missing", "--json"]);
    assert!(!failure.status.success());
    let error: serde_json::Value = serde_json::from_slice(&failure.stderr).unwrap();
    assert_eq!(error["success"], false);
}
#[test]
fn read_only_commands_do_not_create_storage() {
    let parent = tempfile::tempdir().unwrap();
    let dir = parent.path().join("uncreated");
    for args in [
        vec!["--help"],
        vec!["completions", "bash"],
        vec!["list", "--json"],
        vec!["config"],
    ] {
        assert!(run(&dir, &args).status.success());
    }
    assert!(!dir.exists());
}
