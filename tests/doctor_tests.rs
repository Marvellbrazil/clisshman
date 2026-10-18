use clisshman::doctor::DoctorService;
use clisshman::storage::StorageManager;
use tempfile::tempdir;

#[test]
fn test_doctor_diagnostics_report() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().to_path_buf()).unwrap();
    let report = DoctorService::run(&storage);
    assert!(report.config_dir_writable);
    assert!(!storage.config_path().exists());
    assert!(!storage.registry_path().exists());
}
