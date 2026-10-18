use std::io::{stdout, Stdout};
use std::panic::{self, AssertUnwindSafe, PanicHookInfo};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use crossterm::cursor::Show;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::cli::{AddArgs, DuplicateArgs, EditArgs, TunnelAddArgs, TunnelEditArgs};
use crate::model::{Session, Tunnel, TunnelKind};
use crate::service::AppService;
use crate::storage::StorageManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Sessions,
    Tunnels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Reload,
    Save,
    Delete { id: String, view: View },
    Favorite { id: String, value: bool },
    Connect(String),
    RunTunnel(String),
    Ping(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormKind {
    AddSession,
    EditSession,
    DuplicateSession,
    AddTunnel,
    EditTunnel,
}

#[derive(Debug, Clone)]
pub struct Form {
    pub kind: FormKind,
    pub target: Option<String>,
    pub fields: Vec<(&'static str, String)>,
    pub focused: usize,
    pub error: Option<String>,
}

impl Form {
    fn session(kind: FormKind, session: Option<&Session>, port: u16) -> Self {
        let mut fields = vec![
            ("Name", session.map(|s| s.name.clone()).unwrap_or_default()),
            (
                "Group",
                session.and_then(|s| s.group.clone()).unwrap_or_default(),
            ),
            ("Host", session.map(|s| s.host.clone()).unwrap_or_default()),
            (
                "Username",
                session.map(|s| s.user.clone()).unwrap_or_default(),
            ),
            ("Port", session.map(|s| s.port).unwrap_or(port).to_string()),
        ];
        if kind == FormKind::DuplicateSession {
            fields[0].1.push_str("-copy");
            fields[1].0 = "Group (blank keeps source)";
        } else {
            fields.extend([
                (
                    "Identity file",
                    session
                        .and_then(|s| s.identity_file.clone())
                        .unwrap_or_default(),
                ),
                (
                    "Jump host",
                    session
                        .and_then(|s| s.jump_host.clone())
                        .unwrap_or_default(),
                ),
                (
                    "Description",
                    session
                        .and_then(|s| s.description.clone())
                        .unwrap_or_default(),
                ),
                (
                    "Tags (comma separated)",
                    session.map(|s| s.tags.join(", ")).unwrap_or_default(),
                ),
            ]);
        }
        Self {
            kind,
            target: session.map(|s| s.id.clone()),
            fields,
            focused: 0,
            error: None,
        }
    }

    fn tunnel(tunnel: Option<&Tunnel>, session: Option<&str>) -> Self {
        Self {
            kind: if tunnel.is_some() {
                FormKind::EditTunnel
            } else {
                FormKind::AddTunnel
            },
            target: tunnel.map(|t| t.id.clone()),
            fields: vec![
                ("Name", tunnel.map(|t| t.name.clone()).unwrap_or_default()),
                (
                    "Session ID",
                    tunnel
                        .map(|t| t.session_id.clone())
                        .or_else(|| session.map(str::to_owned))
                        .unwrap_or_default(),
                ),
                (
                    "Kind (local/remote/dynamic)",
                    tunnel
                        .map(|t| kind_label(t.kind).to_owned())
                        .unwrap_or_else(|| "local".into()),
                ),
                (
                    "Bind",
                    tunnel
                        .map(|t| t.bind.clone())
                        .unwrap_or_else(|| "127.0.0.1".into()),
                ),
                (
                    "Listen port",
                    tunnel
                        .map(|t| t.listen_port.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "Destination host",
                    tunnel
                        .and_then(|t| t.destination_host.clone())
                        .unwrap_or_default(),
                ),
                (
                    "Destination port",
                    tunnel
                        .and_then(|t| t.destination_port)
                        .map(|p| p.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "Allow public bind (true/false)",
                    tunnel
                        .map(|t| t.allow_public_bind)
                        .unwrap_or(false)
                        .to_string(),
                ),
            ],
            focused: 0,
            error: None,
        }
    }

    fn value(&self, index: usize) -> String {
        self.fields[index].1.trim().to_owned()
    }
    fn optional(&self, index: usize) -> Option<String> {
        let value = self.value(index);
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }
    fn required(&self, index: usize) -> Result<String, String> {
        self.optional(index)
            .ok_or_else(|| format!("{} is required", self.fields[index].0))
    }
    fn port(&self, index: usize) -> Result<u16, String> {
        self.value(index)
            .parse::<u16>()
            .ok()
            .filter(|p| *p > 0)
            .ok_or_else(|| format!("{} must be between 1 and 65535", self.fields[index].0))
    }
    fn target(&self) -> Result<String, String> {
        self.target.clone().ok_or_else(|| "No item selected".into())
    }
    fn tags(&self) -> Vec<String> {
        self.value(8)
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
            .collect()
    }
    fn add_args(&self) -> Result<AddArgs, String> {
        Ok(AddArgs {
            name: Some(self.required(0)?),
            group: self.optional(1),
            host: Some(self.required(2)?),
            username: Some(self.required(3)?),
            port: Some(self.port(4)?),
            identity_file: self.optional(5),
            jump_host: self.optional(6),
            description: self.optional(7),
            tags: self.tags(),
            ..AddArgs::default()
        })
    }
    fn edit_args(&self) -> Result<EditArgs, String> {
        Ok(EditArgs {
            target: self.target()?,
            name: Some(self.required(0)?),
            group: self.optional(1),
            host: Some(self.required(2)?),
            username: Some(self.required(3)?),
            port: Some(self.port(4)?),
            identity_file: self.optional(5),
            jump_host: self.optional(6),
            description: self.optional(7),
            tags: self.tags(),
            clear_group: self.optional(1).is_none(),
            clear_identity: self.optional(5).is_none(),
            clear_jump: self.optional(6).is_none(),
            clear_description: self.optional(7).is_none(),
            clear_tags: self.tags().is_empty(),
            ..EditArgs::default()
        })
    }
    fn duplicate_args(&self) -> Result<DuplicateArgs, String> {
        Ok(DuplicateArgs {
            target: self.target()?,
            name: self.required(0)?,
            group: self.optional(1),
            host: Some(self.required(2)?),
            username: Some(self.required(3)?),
            port: Some(self.port(4)?),
            ..DuplicateArgs::default()
        })
    }
    fn tunnel_args(&self) -> Result<TunnelAddArgs, String> {
        let kind = match self.value(2).to_lowercase().as_str() {
            "local" => TunnelKind::Local,
            "remote" => TunnelKind::Remote,
            "dynamic" => TunnelKind::Dynamic,
            _ => return Err("Kind must be local, remote, or dynamic".into()),
        };
        let allow_public_bind = match self.value(7).to_lowercase().as_str() {
            "true" => true,
            "false" => false,
            _ => return Err("Allow public bind must be true or false".into()),
        };
        if kind == TunnelKind::Dynamic && (self.optional(5).is_some() || self.optional(6).is_some())
        {
            return Err("Clear destination fields for a dynamic tunnel".into());
        }
        Ok(TunnelAddArgs {
            name: self.required(0)?,
            session: self.required(1)?,
            group: None,
            kind,
            bind: self.required(3)?,
            listen_port: self.port(4)?,
            destination_host: if kind == TunnelKind::Dynamic {
                None
            } else {
                Some(self.required(5)?)
            },
            destination_port: if kind == TunnelKind::Dynamic {
                None
            } else {
                Some(self.port(6)?)
            },
            allow_public_bind,
        })
    }
}

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
    default_port: u16,
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
    fn reconcile(&mut self) {
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
    fn move_selection(&mut self, down: bool) {
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

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if key.kind != KeyEventKind::Press {
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.modal.is_some() {
                self.modal = None;
            } else {
                self.should_quit = true;
            }
            return Action::None;
        }
        if let Some(modal) = self.modal.as_mut() {
            match modal {
                Modal::Form(form) => match key.code {
                    KeyCode::Esc => self.modal = None,
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Action::Save
                    }
                    KeyCode::Tab | KeyCode::Down | KeyCode::Enter => {
                        form.focused = (form.focused + 1) % form.fields.len()
                    }
                    KeyCode::BackTab | KeyCode::Up => {
                        form.focused = (form.focused + form.fields.len() - 1) % form.fields.len()
                    }
                    KeyCode::Backspace => {
                        form.fields[form.focused].1.pop();
                        form.error = None;
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        form.fields[form.focused].1.clear();
                        form.error = None;
                    }
                    KeyCode::Char(c)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        form.fields[form.focused].1.push(c);
                        form.error = None;
                    }
                    _ => {}
                },
                Modal::Delete {
                    id,
                    view,
                    confirmed,
                    ..
                } => match key.code {
                    KeyCode::Esc | KeyCode::Char('n') => self.modal = None,
                    KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Tab
                    | KeyCode::BackTab => *confirmed = !*confirmed,
                    KeyCode::Enter => {
                        let action = if *confirmed {
                            Action::Delete {
                                id: id.clone(),
                                view: *view,
                            }
                        } else {
                            Action::None
                        };
                        self.modal = None;
                        return action;
                    }
                    _ => {}
                },
            }
            return Action::None;
        }
        if self.searching {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.searching = false,
                KeyCode::Backspace => {
                    self.search.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.search.clear()
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.search.push(c)
                }
                _ => {}
            }
            return Action::None;
        }
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc if !self.search.is_empty() => self.search.clear(),
            KeyCode::Esc if self.view == View::Tunnels => {
                self.view = View::Sessions;
                self.search.clear();
            }
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('/') => self.searching = true,
            KeyCode::Char('r') => return Action::Reload,
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(true),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(false),
            KeyCode::Char('t') | KeyCode::Tab => {
                self.view = if self.view == View::Sessions {
                    View::Tunnels
                } else {
                    View::Sessions
                };
                self.search.clear();
                self.reconcile();
            }
            KeyCode::Char('F') if self.view == View::Sessions => {
                self.favorites_only = !self.favorites_only
            }
            KeyCode::Char('R') if self.view == View::Sessions => {
                self.recent_only = !self.recent_only
            }
            KeyCode::Char('f') if self.view == View::Sessions => {
                if let Some(s) = self.selected_session() {
                    return Action::Favorite {
                        id: s.id,
                        value: !s.favorite,
                    };
                }
            }
            KeyCode::Char('p') if self.view == View::Sessions => {
                if let Some(s) = self.selected_session() {
                    return Action::Ping(s.id);
                }
            }
            KeyCode::Enter => match self.view {
                View::Sessions => {
                    if let Some(s) = self.selected_session() {
                        return Action::Connect(s.id);
                    }
                }
                View::Tunnels => {
                    if let Some(t) = self.selected_tunnel() {
                        return Action::RunTunnel(t.id);
                    }
                }
            },
            KeyCode::Char('a') => {
                self.modal = Some(Modal::Form(match self.view {
                    View::Sessions => Form::session(FormKind::AddSession, None, self.default_port),
                    View::Tunnels => Form::tunnel(None, self.selected_session_id.as_deref()),
                }));
            }
            KeyCode::Char('e') => match self.view {
                View::Sessions => {
                    if let Some(s) = self.selected_session() {
                        self.modal = Some(Modal::Form(Form::session(
                            FormKind::EditSession,
                            Some(&s),
                            self.default_port,
                        )));
                    }
                }
                View::Tunnels => {
                    if let Some(t) = self.selected_tunnel() {
                        self.modal = Some(Modal::Form(Form::tunnel(Some(&t), None)));
                    }
                }
            },
            KeyCode::Char('d') if self.view == View::Sessions => {
                if let Some(s) = self.selected_session() {
                    self.modal = Some(Modal::Form(Form::session(
                        FormKind::DuplicateSession,
                        Some(&s),
                        self.default_port,
                    )));
                }
            }
            KeyCode::Delete | KeyCode::Char('x') => {
                let item = match self.view {
                    View::Sessions => self.selected_session().map(|s| (s.id, s.name)),
                    View::Tunnels => self.selected_tunnel().map(|t| (t.id, t.name)),
                };
                if let Some((id, name)) = item {
                    self.modal = Some(Modal::Delete {
                        id,
                        name,
                        view: self.view,
                        confirmed: false,
                    });
                }
            }
            _ => {}
        }
        Action::None
    }

    fn accent(&self, color: Color) -> Style {
        if self.no_color {
            Style::default()
        } else {
            Style::default().fg(color)
        }
    }
    fn highlight(&self) -> Style {
        let style = Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD);
        if self.no_color {
            style
        } else {
            style.fg(Color::Cyan)
        }
    }

    pub fn render(&self, frame: &mut Frame<'_>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(3),
                Constraint::Length(3),
            ])
            .split(frame.size());
        let help = match self.view {
            View::Sessions => "a Add | e Edit | d Duplicate | x Delete | f Favorite | F Favorites | R Recent\n/ Search | Enter Connect | p Ping | t Tunnels | r Refresh | q Quit",
            View::Tunnels => "a Add | e Edit | x Delete | Enter Run tunnel\n/ Search | t Sessions | r Refresh | q Quit",
        };
        frame.render_widget(
            Paragraph::new(help)
                .block(Block::default().borders(Borders::ALL).title(format!(
                    " Clisshman | {:?} | Filter: {}{} | Favorites: {} Recent: {} ",
                    self.view,
                    display(&self.search),
                    if self.searching { "_" } else { "" },
                    self.favorites_only,
                    self.recent_only
                )))
                .style(self.accent(Color::Cyan)),
            chunks[0],
        );
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(chunks[1]);
        let (items, selected, details): (Vec<ListItem<'_>>, Option<usize>, String) = match self.view
        {
            View::Sessions => {
                let sessions = self.filtered_sessions();
                let selected = sessions
                    .iter()
                    .position(|s| Some(&s.id) == self.selected_session_id.as_ref());
                let details = self.selected_session().map(|s| format!(
                    "ID: {}\nName: {}\nGroup: {}\nHost: {}\nUser: {}\nPort: {}\nIdentity: {}\nJump: {}\nTags: {}\nDescription: {}\nFavorite: {}\nLast attempt: {}\nCreated: {}\nUpdated: {}",
                    s.id, s.name, s.group.as_deref().unwrap_or("-"), s.host, s.user, s.port, s.identity_file.as_deref().unwrap_or("-"),
                    s.jump_host.as_deref().unwrap_or("-"), s.tags.join(", "), s.description.as_deref().unwrap_or("-"), s.favorite,
                    s.last_connect_attempt_at.map(|v| v.to_rfc3339()).unwrap_or_else(|| "Never".into()), s.created_at.to_rfc3339(), s.updated_at.to_rfc3339()))
                    .unwrap_or_else(|| "No session selected. Use arrows to select; a to add.".into());
                (
                    sessions
                        .iter()
                        .map(|s| {
                            ListItem::new(display(&format!(
                                "{} {} [{}] {}@{}",
                                if s.favorite { "*" } else { " " },
                                s.name,
                                s.group.as_deref().unwrap_or("-"),
                                s.user,
                                s.host
                            )))
                        })
                        .collect(),
                    selected,
                    details,
                )
            }
            View::Tunnels => {
                let tunnels = self.filtered_tunnels();
                let selected = tunnels
                    .iter()
                    .position(|t| Some(&t.id) == self.selected_tunnel_id.as_ref());
                let details = self.selected_tunnel().map(|t| format!("ID: {}\nName: {}\nSession ID: {}\nKind: {}\nBind: {}\nListen port: {}\nDestination: {}:{}\nPublic bind approved: {}\nCreated: {}\nUpdated: {}",
                    t.id, t.name, t.session_id, kind_label(t.kind), t.bind, t.listen_port, t.destination_host.as_deref().unwrap_or("-"),
                    t.destination_port.map(|p| p.to_string()).unwrap_or_else(|| "-".into()), t.allow_public_bind, t.created_at.to_rfc3339(), t.updated_at.to_rfc3339()))
                    .unwrap_or_else(|| "No tunnel selected. Use arrows to select; a to add.".into());
                (
                    tunnels
                        .iter()
                        .map(|t| {
                            ListItem::new(display(&format!(
                                "{} [{}] {}:{}",
                                t.name,
                                kind_label(t.kind),
                                t.bind,
                                t.listen_port
                            )))
                        })
                        .collect(),
                    selected,
                    details,
                )
            }
        };
        let empty = items.is_empty();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {:?} ", self.view)),
            )
            .highlight_style(self.highlight())
            .highlight_symbol("> ");
        let mut list_state = ListState::default();
        list_state.select(selected);
        frame.render_stateful_widget(list, main[0], &mut list_state);
        if empty {
            frame.render_widget(
                Paragraph::new("No matches")
                    .block(Block::default().borders(Borders::ALL).title(" Empty ")),
                main[0],
            );
        }
        frame.render_widget(
            Paragraph::new(display(&details))
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL).title(" Details ")),
            main[1],
        );
        frame.render_widget(
            Paragraph::new(display(&self.status))
                .wrap(Wrap { trim: false })
                .style(self.accent(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title(" Status ")),
            chunks[2],
        );
        if let Some(modal) = &self.modal {
            self.render_modal(frame, modal);
        }
    }

    fn render_modal(&self, frame: &mut Frame<'_>, modal: &Modal) {
        let height = match modal {
            Modal::Form(form) => form.fields.len() as u16 + 6,
            Modal::Delete { .. } => 7,
        };
        let area = popup(frame.size(), height);
        frame.render_widget(Clear, area);
        match modal {
            Modal::Delete {
                name, confirmed, ..
            } => {
                let choices = if *confirmed {
                    "  Cancel     [Delete]"
                } else {
                    " [Cancel]     Delete "
                };
                frame.render_widget(Paragraph::new(format!("Delete {}?\nThis cannot be undone.\n{}\nArrows change choice; Enter confirms; Esc cancels", display(name), choices))
                    .wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL).title(" Confirm deletion ")).style(self.accent(Color::Yellow)), area);
            }
            Modal::Form(form) => {
                let title = match form.kind {
                    FormKind::AddSession => " Add session ",
                    FormKind::EditSession => " Edit session ",
                    FormKind::DuplicateSession => " Duplicate session ",
                    FormKind::AddTunnel => " Add tunnel ",
                    FormKind::EditTunnel => " Edit tunnel ",
                };
                let block = Block::default().borders(Borders::ALL).title(title);
                let inner = block.inner(area);
                frame.render_widget(block, area);
                let sections = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(3)])
                    .split(inner);
                let lines: Vec<Line<'_>> = form
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, (label, value))| {
                        let focused = form.focused == i;
                        Line::from(Span::styled(
                            format!(
                                "{} {}: {}{}",
                                if focused { ">" } else { " " },
                                label,
                                display(value),
                                if focused { "_" } else { "" }
                            ),
                            if focused {
                                self.highlight()
                            } else {
                                Style::default()
                            },
                        ))
                    })
                    .collect();
                let offset = form
                    .focused
                    .saturating_sub(sections[0].height.saturating_sub(1) as usize);
                frame.render_widget(
                    Paragraph::new(lines).scroll((offset as u16, 0)),
                    sections[0],
                );
                let help = vec![
                    Line::from("Tab/Up/Down: field | Ctrl+U: clear"),
                    Line::from("Ctrl+S: Save | Esc: Cancel"),
                    Line::from(Span::styled(
                        form.error.as_deref().map(display).unwrap_or_default(),
                        self.accent(Color::Red),
                    )),
                ];
                frame.render_widget(Paragraph::new(help), sections[1]);
            }
        }
    }
}

pub struct TuiApp {
    service: AppService,
    storage: StorageManager,
    pub state: TuiState,
    ping: Option<mpsc::Receiver<(String, Result<crate::connect::PingResult, String>)>>,
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
            Action::Connect(_) | Action::RunTunnel(_) => {
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

    pub fn run(&mut self) -> Result<(), String> {
        let mut guard = TerminalGuard::new();
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            guard.resume()?;
            let mut terminal =
                Terminal::new(CrosstermBackend::new(stdout())).map_err(|e| e.to_string())?;
            while !self.state.should_quit {
                self.poll_ping();
                terminal
                    .draw(|frame| self.render(frame))
                    .map_err(|e| e.to_string())?;
                if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())? {
                    if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                        let action = self.handle_key(key);
                        match action {
                            Action::Connect(id) => {
                                self.run_connection(&id, false, &mut terminal, &mut guard)?;
                            }
                            Action::RunTunnel(id) => {
                                self.run_connection(&id, true, &mut terminal, &mut guard)?;
                            }
                            action => {
                                let _ = self.apply_action(action);
                            }
                        }
                    }
                }
            }
            Ok(())
        }));
        drop(guard);
        match result {
            Ok(result) => result,
            Err(payload) => panic::resume_unwind(payload),
        }
    }

    fn run_connection(
        &mut self,
        id: &str,
        tunnel: bool,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        guard: &mut TerminalGuard,
    ) -> Result<(), String> {
        guard.suspend()?;
        let result = if tunnel {
            self.service.run_tunnel_returning(id)
        } else {
            self.service.connect_returning(id)
        };
        guard.resume()?;
        terminal.clear().map_err(|e| e.to_string())?;
        self.state.status = match result {
            Ok(code) => format!("SSH exited with status {code}"),
            Err(error) => error,
        };
        if let Err(error) = self.reload() {
            self.state.status = error;
        }
        Ok(())
    }
}

fn kind_label(kind: TunnelKind) -> &'static str {
    match kind {
        TunnelKind::Local => "local",
        TunnelKind::Remote => "remote",
        TunnelKind::Dynamic => "dynamic",
    }
}
fn display(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .collect()
}
fn popup(area: Rect, height: u16) -> Rect {
    let width = area.width.saturating_sub(4).min(90);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}
fn restore_terminal() -> Result<(), String> {
    let raw = disable_raw_mode();
    let screen = execute!(stdout(), LeaveAlternateScreen, Show);
    raw.and(screen).map_err(|e| e.to_string())
}

type PanicHook = Arc<dyn Fn(&PanicHookInfo<'_>) + Send + Sync + 'static>;
struct TerminalGuard {
    previous_hook: Option<PanicHook>,
}
impl TerminalGuard {
    fn new() -> Self {
        let previous_hook: PanicHook = Arc::from(panic::take_hook());
        let hook = Arc::clone(&previous_hook);
        let thread = std::thread::current().id();
        panic::set_hook(Box::new(move |info| {
            if std::thread::current().id() == thread {
                let _ = restore_terminal();
            }
            hook(info);
        }));
        Self {
            previous_hook: Some(previous_hook),
        }
    }
    fn resume(&mut self) -> Result<(), String> {
        enable_raw_mode().map_err(|e| e.to_string())?;
        execute!(stdout(), EnterAlternateScreen).map_err(|e| e.to_string())
    }
    fn suspend(&mut self) -> Result<(), String> {
        restore_terminal()
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore_terminal();
        if !std::thread::panicking() {
            if let Some(hook) = self.previous_hook.take() {
                panic::set_hook(Box::new(move |info| hook(info)));
            }
        }
    }
}
