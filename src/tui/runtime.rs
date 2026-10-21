use super::terminal::TerminalGuard;
use super::{Action, TuiApp};
use crossterm::event::{self, Event};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Stdout};
use std::panic::{self, AssertUnwindSafe};
use std::time::Duration;

impl TuiApp {
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
                            Action::Transfer(id) => {
                                guard.suspend()?;
                                let result = crate::wizard::transfer(&self.service, &id);
                                guard.resume()?;
                                terminal.clear().map_err(|e| e.to_string())?;
                                self.state.status = match result {
                                    Ok(code) => {
                                        format!("SCP finished with status {code} (or cancelled)")
                                    }
                                    Err(error) => error,
                                };
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
