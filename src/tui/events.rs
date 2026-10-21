use super::{Action, Form, FormKind, Modal, TuiState, View};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

impl TuiState {
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
            KeyCode::Char('s') if self.view == View::Sessions => {
                if let Some(session) = self.selected_session() {
                    return Action::Transfer(session.id);
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
}
