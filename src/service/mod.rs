mod configuration;
mod connections;
mod sessions;
mod tunnels;
pub use crate::selection::{resolve_session, resolve_session_index};
use crate::storage::StorageManager;
pub use tunnels::tunnel_args;
#[derive(Clone)]
pub struct AppService {
    pub storage: StorageManager,
}

impl AppService {
    pub fn new(storage: StorageManager) -> Self {
        Self { storage }
    }
}
