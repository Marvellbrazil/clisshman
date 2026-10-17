use std::io::{self, stdout, Stdout};
use std::time::Duration;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;

use crate::cli::ConnectArgs;
use crate::model::Session;
use crate::service::AppService;
use crate::storage::StorageManager;

pub struct TuiApp {
    service: AppService,
    storage: StorageManager,
    sessions: Vec<Session>,
    state: ListState,
    search: String,
    searching: bool,
    status_msg: String,
    should_quit: bool,
}

impl TuiApp {
    pub fn new(service: AppService, storage: StorageManager) -> Result<Self, String> {
        let reg = storage.load_registry()?;
        let mut state = ListState::default();
        if !reg.sessions.is_empty() {
            state.select(Some(0));
        }
        Ok(Self {
            service,
            storage,
            sessions: reg.sessions,
            state,
            search: String::new(),
            searching: false,
            status_msg: "Press Enter to connect, '/' to search, 'p' to ping, 'q' to quit".to_string(),
            should_quit: false,
        })
    }

    pub fn filtered_sessions(&self) -> Vec<Session> {
        if self.search.trim().is_empty() {
            self.sessions.clone()
        } else {
            let term = self.search.trim().to_lowercase();
            self.sessions
                .iter()
                .filter(|s| {
                    s.name.to_lowercase().contains(&term)
                        || s.host.to_lowercase().contains(&term)
                        || s.user.to_lowercase().contains(&term)
                        || s.group.as_deref().unwrap_or("").to_lowercase().contains(&term)
                        || s.tags.iter().any(|t| t.to_lowercase().contains(&term))
                })
                .cloned()
                .collect()
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        enable_raw_mode().map_err(|e| e.to_string())?;
        let mut stdout_handle = stdout();
        execute!(stdout_handle, EnterAlternateScreen).map_err(|e| e.to_string())?;
        let backend = CrosstermBackend::new(stdout_handle);
        let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;

        let res = self.run_event_loop(&mut terminal);

        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        let _ = terminal.show_cursor();

        res
    }

    fn run_event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), String> {
        while !self.should_quit {
            let filtered = self.filtered_sessions();
            let selected_idx = self.state.selected();

            let detail_text = if let Some(idx) = selected_idx {
                if idx < filtered.len() {
                    let s = &filtered[idx];
                    format!(
                        "ID:          {}\nName:        {}\nGroup:       {}\nHost:        {}\nUser:        {}\nPort:        {}\nIdentity:    {}\nJump Host:   {}\nTags:        {}\nDescription: {}\nCreated At:  {}\nUpdated At:  {}",
                        s.id,
                        s.name,
                        s.group.as_deref().unwrap_or("-"),
                        s.host,
                        s.user,
                        s.port,
                        s.identity_file.as_deref().unwrap_or("-"),
                        s.jump_host.as_deref().unwrap_or("-"),
                        s.tags.join(", "),
                        s.description.as_deref().unwrap_or("-"),
                        s.created_at.to_rfc3339(),
                        s.updated_at.to_rfc3339(),
                    )
                } else {
                    "No session selected".to_string()
                }
            } else {
                "No session selected".to_string()
            };

            terminal
                .draw(|f| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3),
                            Constraint::Min(5),
                            Constraint::Length(3),
                        ])
                        .split(f.size());

                    let title_text = format!(
                        " Clisshman TUI | Filter: {} ",
                        if self.searching {
                            format!("{}_", self.search)
                        } else if self.search.is_empty() {
                            "(All)".to_string()
                        } else {
                            self.search.clone()
                        }
                    );
                    let title_block = Block::default()
                        .borders(Borders::ALL)
                        .title(title_text)
                        .style(Style::default().fg(Color::Cyan));
                    let title_p = Paragraph::new(
                        "[/] Search | [Enter] Connect | [p] Ping | [r] Refresh | [q] Quit",
                    )
                    .block(title_block);
                    f.render_widget(title_p, chunks[0]);

                    let main_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                        .split(chunks[1]);

                    let items: Vec<ListItem> = filtered
                        .iter()
                        .map(|s| {
                            let label = format!(
                                "{} [{}] -> {}@{}",
                                s.name,
                                s.group.as_deref().unwrap_or("-"),
                                s.user,
                                s.host
                            );
                            ListItem::new(label)
                        })
                        .collect();

                    let list_block = Block::default()
                        .borders(Borders::ALL)
                        .title(" Sessions ")
                        .style(Style::default().fg(Color::White));
                    let list = List::new(items)
                        .block(list_block)
                        .highlight_style(
                            Style::default()
                                .bg(Color::Cyan)
                                .fg(Color::Black)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol("▶ ");
                    f.render_stateful_widget(list, main_chunks[0], &mut self.state);

                    let detail_block = Block::default()
                        .borders(Borders::ALL)
                        .title(" Session Details ")
                        .style(Style::default().fg(Color::White));
                    let detail_p = Paragraph::new(detail_text).block(detail_block);
                    f.render_widget(detail_p, main_chunks[1]);

                    let status_block = Block::default()
                        .borders(Borders::ALL)
                        .title(" Status ")
                        .style(Style::default().fg(Color::Yellow));
                    let status_p = Paragraph::new(self.status_msg.clone()).block(status_block);
                    f.render_widget(status_p, chunks[2]);
                })
                .map_err(|e| e.to_string())?;

            if event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())? {
                if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                    if key.kind == KeyEventKind::Press {
                        if self.searching {
                            match key.code {
                                KeyCode::Enter | KeyCode::Esc => {
                                    self.searching = false;
                                }
                                KeyCode::Backspace => {
                                    self.search.pop();
                                }
                                KeyCode::Char(c) => {
                                    self.search.push(c);
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    self.should_quit = true;
                                }
                                KeyCode::Char('/') => {
                                    self.searching = true;
                                }
                                KeyCode::Char('r') => {
                                    let _ = self.reload_sessions();
                                    self.status_msg = "Sessions reloaded".to_string();
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    self.next_item();
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    self.previous_item();
                                }
                                KeyCode::Enter => {
                                    self.connect_selected(terminal)?;
                                }
                                KeyCode::Char('p') => {
                                    self.ping_selected();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn reload_sessions(&mut self) -> Result<(), String> {
        let reg = self.storage.load_registry()?;
        self.sessions = reg.sessions;
        Ok(())
    }

    fn next_item(&mut self) {
        let count = self.filtered_sessions().len();
        if count == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous_item(&mut self) {
        let count = self.filtered_sessions().len();
        if count == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn connect_selected(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), String> {
        let filtered = self.filtered_sessions();
        if let Some(idx) = self.state.selected() {
            if idx < filtered.len() {
                let session = filtered[idx].clone();

                disable_raw_mode().map_err(|e| e.to_string())?;
                execute!(stdout(), LeaveAlternateScreen).map_err(|e| e.to_string())?;
                terminal.show_cursor().map_err(|e| e.to_string())?;

                println!("Connecting to {} ({})...", session.name, session.host);
                let _ = self.service.handle_connect(ConnectArgs {
                    target: session.id,
                    group: None,
                    dry_run: false,
                    passthrough: vec![],
                });

                println!("\nSession ended. Press Enter to return to Clisshman TUI...");
                let mut buf = String::new();
                let _ = io::stdin().read_line(&mut buf);

                enable_raw_mode().map_err(|e| e.to_string())?;
                execute!(stdout(), EnterAlternateScreen).map_err(|e| e.to_string())?;
                terminal.clear().map_err(|e| e.to_string())?;
                self.status_msg = "Returned from SSH connection".to_string();
            }
        }
        Ok(())
    }

    fn ping_selected(&mut self) {
        let filtered = self.filtered_sessions();
        if let Some(idx) = self.state.selected() {
            if idx < filtered.len() {
                let s = &filtered[idx];
                let res = crate::connect::PingService::ping_tcp(
                    &s.host,
                    s.port,
                    std::time::Duration::from_secs(2),
                );
                self.status_msg = format!("Ping {}: {}", if res.success { "OK" } else { "FAIL" }, res.message);
            }
        }
    }
}
