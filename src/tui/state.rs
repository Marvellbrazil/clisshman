use super::render::kind_label;
use super::{Form, View};
use crate::model::{Session, Tunnel};

#[derive(Debug, Clone)]
pub enum Modal {
    Form(Form),
    Delete {
        id: String,
        name: String,
        view: View,
        confirmed: bool,
    },
}

#[derive(Debug, Clone)]
pub struct TuiState {
    pub sessions: Vec<Session>,
    pub tunnels: Vec<Tunnel>,
    pub selected_session_id: Option<String>,
    pub selected_tunnel_id: Option<String>,
    pub search: String,
    pub searching: bool,
    pub favorites_only: bool,
    pub recent_only: bool,
    pub view: View,
    pub modal: Option<Modal>,
    pub status: String,
    pub should_quit: bool,
    pub no_color: bool,
    pub(super) default_port: u16,
}

impl TuiState {
    pub fn new(
        sessions: Vec<Session>,
        tunnels: Vec<Tunnel>,
        default_port: u16,
        no_color: bool,
    ) -> Self {
        let mut state = Self {
            sessions,
            tunnels,
            selected_session_id: None,
            selected_tunnel_id: None,
            search: String::new(),
            searching: false,
            favorites_only: false,
            recent_only: false,
            view: View::Sessions,
            modal: None,
            status: "Ready".into(),
            should_quit: false,
            no_color,
            default_port,
        };
        state.reconcile();
        state
    }

    pub fn filtered_sessions(&self) -> Vec<Session> {
        crate::selection::ranked(
            self.sessions
                .iter()
                .filter(|s| {
                    (!self.favorites_only || s.favorite)
                        && (!self.recent_only || s.last_connect_attempt_at.is_some())
                })
                .cloned()
                .collect(),
            &self.search,
        )
    }

    pub fn filtered_tunnels(&self) -> Vec<Tunnel> {
        let query = self.search.trim().to_lowercase();
        self.tunnels
            .iter()
            .filter(|t| {
                let session_name = self
                    .sessions
                    .iter()
                    .find(|s| s.id == t.session_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("");
                query.is_empty()
                    || format!(
                        "{} {} {} {} {}",
                        t.id,
                        t.name,
                        t.session_id,
                        session_name,
                        kind_label(t.kind)
                    )
                    .to_lowercase()
                    .contains(&query)
            })
            .cloned()
            .collect()
    }

    pub fn selected_session(&self) -> Option<Session> {
        self.filtered_sessions()
            .into_iter()
            .find(|s| Some(&s.id) == self.selected_session_id.as_ref())
    }
    pub fn selected_tunnel(&self) -> Option<Tunnel> {
        self.filtered_tunnels()
            .into_iter()
            .find(|t| Some(&t.id) == self.selected_tunnel_id.as_ref())
    }
    pub fn replace_items(&mut self, sessions: Vec<Session>, tunnels: Vec<Tunnel>) {
        self.sessions = sessions;
        self.tunnels = tunnels;
        self.reconcile();
    }
    pub(super) fn reconcile(&mut self) {
        if !self
            .sessions
            .iter()
            .any(|s| Some(&s.id) == self.selected_session_id.as_ref())
        {
            self.selected_session_id = self.filtered_sessions().first().map(|s| s.id.clone());
        }
        if !self
            .tunnels
            .iter()
            .any(|t| Some(&t.id) == self.selected_tunnel_id.as_ref())
        {
            self.selected_tunnel_id = self.filtered_tunnels().first().map(|t| t.id.clone());
        }
    }
    pub(super) fn move_selection(&mut self, down: bool) {
        let ids: Vec<String> = match self.view {
            View::Sessions => self.filtered_sessions().into_iter().map(|s| s.id).collect(),
            View::Tunnels => self.filtered_tunnels().into_iter().map(|t| t.id).collect(),
        };
        if ids.is_empty() {
            return;
        }
        let selected = match self.view {
            View::Sessions => &mut self.selected_session_id,
            View::Tunnels => &mut self.selected_tunnel_id,
        };
        let index = ids.iter().position(|id| Some(id) == selected.as_ref());
        let next = match index {
            Some(i) if down => (i + 1) % ids.len(),
            Some(0) => ids.len() - 1,
            Some(i) => i - 1,
            None => 0,
        };
        *selected = ids.get(next).cloned();
    }
}
