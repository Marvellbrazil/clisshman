use tempfile::tempdir;
use clisshman::doctor::DoctorService;
use clisshman::storage::StorageManager;

#[test]
fn test_doctor_diagnostics_report() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().to_path_buf()).unwrap();
    let report = DoctorService::run(&storage);
    assert!(report.config_dir_writable);
}
