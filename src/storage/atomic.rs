use super::StorageManager;
#[cfg(unix)]
use std::fs::File;
use std::io::Write;
use std::path::Path;
impl StorageManager {
    pub fn atomic_write(path: &Path, bytes: &[u8], overwrite: bool) -> Result<(), String> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut temporary = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| "Cannot create temporary output file")?;
        temporary
            .write_all(bytes)
            .map_err(|_| "Cannot write temporary output file")?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|_| "Cannot sync output file")?;
        if overwrite {
            temporary
                .persist(path)
                .map_err(|_| "Cannot replace output file")?;
        } else {
            temporary
                .persist_noclobber(path)
                .map_err(|_| "Output already exists or cannot be created")?;
        }
        #[cfg(unix)]
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| "Cannot sync output directory")?;
        Ok(())
    }
}
