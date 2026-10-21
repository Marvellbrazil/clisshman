use super::{Action, FormKind, Modal, TuiState, View};
use crate::cli::TunnelEditArgs;
use crate::model::Session;
use crate::service::AppService;
use crate::storage::StorageManager;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use std::sync::mpsc;

pub struct TuiApp {
    pub(super) service: AppService,
    pub(super) storage: StorageManager,
    pub state: TuiState,
    pub(super) ping: Option<mpsc::Receiver<(String, Result<crate::connect::PingResult, String>)>>,
}

impl TuiApp {
    pub fn new(service: AppService, storage: StorageManager) -> Result<Self, String> {
        let registry = storage.load_registry()?;
        let config = storage.load_config()?;
        Ok(Self {
            service,
            storage,
            state: TuiState::new(
                registry.sessions,
                registry.tunnels,
                config.default_port,
                std::env::var_os("NO_COLOR").is_some(),
            ),
            ping: None,
        })
    }
    pub fn filtered_sessions(&self) -> Vec<Session> {
        self.state.filtered_sessions()
    }
    pub fn reload(&mut self) -> Result<(), String> {
        let registry = self.storage.load_registry()?;
        self.state
            .replace_items(registry.sessions, registry.tunnels);
        Ok(())
    }
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        self.state.handle_key(key)
    }
    pub fn render(&self, frame: &mut Frame<'_>) {
        self.state.render(frame);
    }

    pub fn apply_action(&mut self, action: Action) -> Result<(), String> {
        let result = match action {
            Action::None => return Ok(()),
            Action::Reload => self.reload().map(|()| "Reloaded".to_owned()),
            Action::Save => self.save_form(),
            Action::Delete { id, view } => match view {
                View::Sessions => self
                    .service
                    .delete(&id, None)
                    .map(|s| format!("Deleted {}", s.name)),
                View::Tunnels => self
                    .service
                    .delete_tunnel(&id)
                    .map(|t| format!("Deleted tunnel {}", t.name)),
            },
            Action::Favorite { id, value } => self
                .service
                .favorite(&id, None, value)
                .map(|s| format!("Favorite {}: {}", s.name, value)),
            Action::Ping(id) => {
                self.start_ping(id);
                return Ok(());
            }
            Action::Connect(_) | Action::Transfer(_) | Action::RunTunnel(_) => {
                return Err("Connection requires a suspended terminal".into())
            }
        };
        match result {
            Ok(message) => {
                self.state.status = message;
                if let Err(error) = self.reload() {
                    self.state.status = error.clone();
                    return Err(error);
                }
                Ok(())
            }
            Err(error) => {
                if let Some(Modal::Form(form)) = self.state.modal.as_mut() {
                    form.error = Some(error.clone());
                }
                self.state.status = error.clone();
                Err(error)
            }
        }
    }

    fn save_form(&mut self) -> Result<String, String> {
        let form = match &self.state.modal {
            Some(Modal::Form(form)) => form.clone(),
            _ => return Err("No form open".into()),
        };
        match form.kind {
            FormKind::AddSession | FormKind::EditSession | FormKind::DuplicateSession => {
                let session = match form.kind {
                    FormKind::AddSession => self.service.add(form.add_args()?)?,
                    FormKind::EditSession => self.service.edit(form.edit_args()?)?,
                    _ => self.service.duplicate(form.duplicate_args()?)?,
                };
                self.state.selected_session_id = Some(session.id);
            }
            FormKind::AddTunnel | FormKind::EditTunnel => {
                let args = form.tunnel_args()?;
                if form.kind == FormKind::EditTunnel && !args.allow_public_bind {
                    let old = self.service.get_tunnel(&form.target()?)?;
                    if old.allow_public_bind && old.bind == args.bind {
                        return Err(
                            "Change bind to a loopback address to revoke public binding".into()
                        );
                    }
                }
                let tunnel = if form.kind == FormKind::AddTunnel {
                    self.service.add_tunnel(args)?
                } else {
                    self.service.edit_tunnel(TunnelEditArgs {
                        target: form.target()?,
                        name: Some(args.name),
                        session: Some(args.session),
                        group: None,
                        kind: Some(args.kind),
                        bind: Some(args.bind),
                        listen_port: Some(args.listen_port),
                        destination_host: args.destination_host,
                        destination_port: args.destination_port,
                        allow_public_bind: args.allow_public_bind,
                    })?
                };
                self.state.selected_tunnel_id = Some(tunnel.id);
            }
        }
        self.state.modal = None;
        Ok("Saved".into())
    }

    fn start_ping(&mut self, id: String) {
        if self.ping.is_some() {
            self.state.status = "Ping already running".into();
            return;
        }
        let service = self.service.clone();
        let (sender, receiver) = mpsc::channel();
        self.state.status = format!("Pinging session {id}...");
        self.ping = Some(receiver);
        std::thread::spawn(move || {
            let result = service.ping(&id, false, Some(2));
            let _ = sender.send((id, result));
        });
    }
    pub fn poll_ping(&mut self) {
        let result = match self.ping.as_ref().map(mpsc::Receiver::try_recv) {
            Some(Ok(result)) => Some(result),
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.ping = None;
                self.state.status = "Ping worker stopped".into();
                None
            }
            _ => None,
        };
        if let Some((id, result)) = result {
            self.ping = None;
            self.state.status = match result {
                Ok(result) if result.success => format!(
                    "Ping {id}: OK ({} ms)",
                    result.latency_ms.unwrap_or_default()
                ),
                Ok(_) => format!("Ping {id}: unreachable"),
                Err(_) => format!("Ping {id}: unable to check connection"),
            };
        }
    }
}
