use super::{FormKind, Modal, TuiState, View};
use crate::model::TunnelKind;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

impl TuiState {
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
            View::Sessions => "a Add | e Edit | d Duplicate | x Delete | f Favorite | F Favorites | R Recent\n/ Search | Enter Connect | s SCP | p Ping | t Tunnels | r Refresh | q Quit",
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

pub(super) fn kind_label(kind: TunnelKind) -> &'static str {
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
