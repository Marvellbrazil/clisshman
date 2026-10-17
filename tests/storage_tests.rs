#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use clisshman::config::IdType;
    use clisshman::model::{normalize_tags, Session};
    use clisshman::storage::StorageManager;
    use chrono::Utc;

    #[test]
    fn test_normalize_tags_alphabetical_and_trimmed() {
        let raw = vec![
            "work, Debian, Web Server".to_string(),
            " debian , api ".to_string(),
        ];
        let normalized = normalize_tags(&raw);
        assert_eq!(normalized, vec!["Debian", "Web Server", "api", "debian", "work"]);
    }

    #[test]
    fn test_storage_atomic_save_and_load() {
        let dir = tempdir().unwrap();
        let storage = StorageManager::with_dir(dir.path().to_path_buf()).unwrap();

        let cfg = storage.load_config().unwrap();
        assert_eq!(cfg.id_type, IdType::Integer);
        assert_eq!(cfg.default_port, 22);

        let mut reg = storage.load_registry().unwrap();
        assert_eq!(reg.sessions.len(), 0);

        let session = Session {
            id: "1".to_string(),
            group: Some("prod".to_string()),
            name: "web-01".to_string(),
            host: "10.0.0.1".to_string(),
            user: "root".to_string(),
            port: 22,
            identity_file: None,
            jump_host: None,
            description: None,
            tags: vec!["api".to_string(), "prod".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        reg.sessions.push(session.clone());
        reg.next_id = 2;
        storage.save_registry(&reg).unwrap();

        let loaded = storage.load_registry().unwrap();
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].name, "web-01");
        assert_eq!(loaded.next_id, 2);
    }
}
