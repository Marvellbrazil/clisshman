use crossterm::cursor::Show;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::io::stdout;
use std::panic::{self, PanicHookInfo};
use std::sync::Arc;

fn restore_terminal() -> Result<(), String> {
    let raw = disable_raw_mode();
    let screen = execute!(stdout(), LeaveAlternateScreen, Show);
    raw.and(screen).map_err(|e| e.to_string())
}

type PanicHook = Arc<dyn Fn(&PanicHookInfo<'_>) + Send + Sync + 'static>;
pub(super) struct TerminalGuard {
    previous_hook: Option<PanicHook>,
}
impl TerminalGuard {
    pub(super) fn new() -> Self {
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
    pub(super) fn resume(&mut self) -> Result<(), String> {
        enable_raw_mode().map_err(|e| e.to_string())?;
        execute!(stdout(), EnterAlternateScreen).map_err(|e| e.to_string())
    }
    pub(super) fn suspend(&mut self) -> Result<(), String> {
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
