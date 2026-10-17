use chrono::Utc;
use tempfile::tempdir;
use clisshman::model::Session;
use clisshman::service::AppService;
use clisshman::storage::StorageManager;
use clisshman::tui::TuiApp;

#[test]
fn test_tui_app_filter_logic() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().to_path_buf()).unwrap();

    let mut reg = storage.load_registry().unwrap();
    reg.sessions.push(Session {
        id: "1".to_string(),
        group: Some("production".to_string()),
        name: "api-gateway".to_string(),
        host: "10.0.1.1".to_string(),
        user: "ubuntu".to_string(),
        port: 22,
        identity_file: None,
        jump_host: None,
        description: None,
        tags: vec!["gateway".to_string(), "prod".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });
    storage.save_registry(&reg).unwrap();

    let service = AppService::new(storage.clone());
    let app = TuiApp::new(service, storage).unwrap();

    let filtered = app.filtered_sessions();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "api-gateway");
}
