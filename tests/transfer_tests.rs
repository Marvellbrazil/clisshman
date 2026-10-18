use clisshman::{
    cli::{AddArgs, ScpArgs},
    service::AppService,
    storage::StorageManager,
    transfer::TransferPlan,
};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn request(file: PathBuf, to: PathBuf) -> ScpArgs {
    ScpArgs {
        target: "1".into(),
        group: None,
        files: vec![file],
        to,
        download: false,
        recursive: false,
        yes: false,
        dry_run: false,
    }
}
fn service(dir: &std::path::Path) -> AppService {
    let service = AppService::new(StorageManager::with_dir(dir.join("config")).unwrap());
    service
        .add(AddArgs {
            host: Some("::1".into()),
            username: Some("deploy".into()),
            port: Some(2222),
            identity_file: Some("~/my key".into()),
            jump_host: Some("bastion".into()),
            ..Default::default()
        })
        .unwrap();
    service
}
#[test]
fn upload_args_keep_native_options_before_literal_operands() {
    let dir = tempfile::tempdir().unwrap();
    let service = service(dir.path());
    let source = dir.path().join("report, final.txt");
    std::fs::write(&source, "sample").unwrap();
    let plan = TransferPlan::new(
        &service.get("1", None).unwrap(),
        &request(source.clone(), "/uploads dir".into()),
    )
    .unwrap();
    let args: Vec<_> = plan.args.iter().map(|a| a.to_string_lossy()).collect();
    assert_eq!(&args[..2], ["-P", "2222"]);
    assert!(args.iter().any(|v| v == "-J"));
    assert!(args.iter().any(|v| v.ends_with("my key")));
    assert_eq!(args.last().unwrap(), "deploy@[::1]:/uploads dir/");
    assert!(args[args.len() - 2].ends_with("report, final.txt"));
    assert_eq!(args[args.len() - 3], "--");
}
#[test]
fn folders_need_recursive_and_download_needs_existing_directory() {
    let dir = tempfile::tempdir().unwrap();
    let service = service(dir.path());
    let session = service.get("1", None).unwrap();
    let mut args = request(dir.path().into(), "relative dir".into());
    assert!(TransferPlan::new(&session, &args).is_err());
    args.recursive = true;
    assert_eq!(
        TransferPlan::new(&session, &args)
            .unwrap()
            .args
            .last()
            .unwrap(),
        "deploy@[::1]:./relative dir/"
    );
    args.download = true;
    args.files = vec!["/logs/app.log".into(), "/logs/other.log".into()];
    args.to = dir.path().into();
    let plan = TransferPlan::new(&session, &args).unwrap();
    assert!(plan.args.iter().any(|a| a == "deploy@[::1]:/logs/app.log"));
    args.to = dir.path().join("missing");
    assert!(TransferPlan::new(&session, &args).is_err());
}
#[test]
fn remote_shell_syntax_and_missing_local_sources_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let service = service(dir.path());
    let session = service.get("1", None).unwrap();
    for path in [
        "/tmp/*",
        "/tmp/$(touch bad)",
        "/tmp/abc\nother",
        "/tmp/foo;bar",
    ] {
        let mut args = request(path.into(), dir.path().into());
        args.download = true;
        assert!(TransferPlan::new(&session, &args).is_err());
    }
    assert!(
        TransferPlan::new(&session, &request(dir.path().join("absent"), "/tmp".into())).is_err()
    );
}
#[test]
fn cli_confirmation_dry_run_alias_and_completions() {
    let dir = tempfile::tempdir().unwrap();
    let service = service(dir.path());
    let source = dir.path().join("local file.txt");
    std::fs::write(&source, "test").unwrap();
    let run = |exe: &str, extra: &[&str]| {
        Command::new(exe)
            .env(
                "CLISSHMAN_CONFIG_DIR",
                service.storage.config_path().parent().unwrap(),
            )
            .args(extra)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let exe = env!("CARGO_BIN_EXE_clisshman");
    let args = [
        "scp",
        "1",
        "--file",
        source.to_str().unwrap(),
        "--to",
        "/tmp",
        "--dry-run",
        "--json",
    ];
    let before = std::fs::read(service.storage.registry_path()).unwrap();
    let dry = run(exe, &args);
    assert!(
        dry.status.success(),
        "{}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let preview: serde_json::Value = serde_json::from_slice(&dry.stdout).unwrap();
    assert_eq!(preview["program"], "scp");
    assert_eq!(
        std::fs::read(service.storage.registry_path()).unwrap(),
        before
    );
    let transfer = run(exe, &args[..6]);
    assert!(!transfer.status.success());
    assert!(String::from_utf8_lossy(&transfer.stderr).contains("--yes"));
    assert_eq!(
        std::fs::read(service.storage.registry_path()).unwrap(),
        before
    );
    let alias = run(env!("CARGO_BIN_EXE_csm"), &["list", "--json"]);
    assert!(alias.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&alias.stdout).unwrap()[0]["id"],
        "1"
    );
    let completions = run(
        env!("CARGO_BIN_EXE_csm"),
        &["completions", "bash", "--command-name", "csm"],
    );
    assert!(completions.status.success());
    assert!(String::from_utf8_lossy(&completions.stdout).contains(" csm"));
}
#[test]
fn missing_native_client_returns_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let plan = TransferPlan {
        args: vec![],
        download: false,
    };
    assert!(plan
        .execute_program(dir.path().join("missing-scp").as_os_str())
        .is_err());
}

#[cfg(windows)]
#[test]
fn runner_forwards_windows_status_without_network() {
    let program = std::env::var_os("COMSPEC").unwrap();
    let plan = TransferPlan {
        args: vec!["/D".into(), "/C".into(), "exit 7".into()],
        download: false,
    };
    assert_eq!(plan.execute_program(&program).unwrap(), 7);
}

#[cfg(unix)]
#[test]
fn runner_forwards_native_status_without_network() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join("fake-scp");
    std::fs::write(&program, "#!/bin/sh\nexit 7\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let plan = TransferPlan {
        args: vec![],
        download: false,
    };
    assert_eq!(plan.execute_program(program.as_os_str()).unwrap(), 7);
}
