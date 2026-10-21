mod export;
mod parser;
use crate::{
    config::IdType,
    model::{generate_id, Registry},
    validation,
};
use std::path::Path;
pub struct SshConfigManager;
impl SshConfigManager {
    pub fn import_from_file<P: AsRef<Path>>(
        path: P,
        registry: &mut Registry,
        id_type: IdType,
    ) -> Result<usize, String> {
        let mut sessions = Self::parse_from_file(path)?;
        for session in &mut sessions {
            validation::session(session)?;
            if registry
                .sessions
                .iter()
                .any(|existing| validation::same_profile(existing, session))
            {
                return Err(format!("Session '{}' already exists", session.name));
            }
        }
        let count = sessions.len();
        for mut session in sessions {
            session.id = generate_id(id_type, &mut registry.next_id)?;
            registry.sessions.push(session);
        }
        Ok(count)
    }
}
