use chrono::Utc;
use clisshman::model::{Session, Tunnel, TunnelKind};
use clisshman::service::AppService;
use clisshman::storage::StorageManager;
use clisshman::tui::{Action, FormKind, Modal, TuiApp, TuiState, View};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use tempfile::tempdir;

fn session(id: &str, name: &str) -> Session {
    Session {
        id: id.into(),
        group: Some("production".into()),
        name: name.into(),
        host: "127.0.0.1".into(),
        user: "ubuntu".into(),
        port: 22,
        identity_file: None,
        jump_host: None,
        description: None,
        tags: vec!["prod".into()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        favorite: false,
        last_connect_attempt_at: None,
    }
}
fn tunnel(id: &str, session_id: &str) -> Tunnel {
    Tunnel {
        id: id.into(),
        name: "database".into(),
        session_id: session_id.into(),
        kind: TunnelKind::Local,
        bind: "127.0.0.1".into(),
        listen_port: 5433,
        destination_host: Some("localhost".into()),
        destination_port: Some(5432),
        allow_public_bind: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}
fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
fn state() -> TuiState {
    TuiState::new(
        vec![session("1", "alpha"), session("2", "beta")],
        vec![],
        22,
        true,
    )
}
fn form_field(app: &mut TuiApp, index: usize, value: &str) {
    match app.state.modal.as_mut() {
        Some(Modal::Form(form)) => form.fields[index].1 = value.into(),
        _ => panic!("Expected native form"),
    }
}
fn apply(app: &mut TuiApp, code: KeyCode) {
    let action = app.handle_key(key(code));
    app.apply_action(action).unwrap();
}
fn save(app: &mut TuiApp) -> Result<(), String> {
    let action = app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
    assert_eq!(action, Action::Save);
    app.apply_action(action)
}
fn rendered(state: &TuiState, width: u16, height: u16) -> (String, Terminal<TestBackend>) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| state.render(frame)).unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    (text, terminal)
}

#[test]
fn test_tui_app_filter_logic() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().to_path_buf()).unwrap();
    let mut reg = storage.load_registry().unwrap();
    reg.sessions.push(session("1", "api-gateway"));
    reg.next_id = 2;
    storage
        .update_registry(|registry| {
            *registry = reg;
            Ok(())
        })
        .unwrap();
    let app = TuiApp::new(AppService::new(storage.clone()), storage).unwrap();
    assert_eq!(app.filtered_sessions()[0].name, "api-gateway");
}

#[test]
fn pure_search_preserves_id_and_never_connects_a_stale_index() {
    let mut state = state();
    state.selected_session_id = Some("2".into());
    state.search = "alpha".into();
    assert!(state.selected_session().is_none());
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Action::None);
    assert_eq!(state.selected_session_id.as_deref(), Some("2"));
    state.search.clear();
    assert_eq!(
        state.handle_key(key(KeyCode::Enter)),
        Action::Connect("2".into())
    );
    state.replace_items(vec![session("2", "beta"), session("1", "alpha")], vec![]);
    assert_eq!(state.selected_session().unwrap().id, "2");
    state.replace_items(vec![session("1", "alpha")], vec![]);
    assert_eq!(state.selected_session().unwrap().id, "1");
}

#[test]
fn pure_empty_navigation_and_repeat_events_are_safe() {
    let mut state = TuiState::new(vec![], vec![], 22, true);
    for code in [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Char('j'),
        KeyCode::Char('k'),
        KeyCode::Enter,
        KeyCode::Char('x'),
    ] {
        assert_eq!(state.handle_key(key(code)), Action::None);
    }
    let mut repeat = key(KeyCode::Char('q'));
    repeat.kind = KeyEventKind::Release;
    state.handle_key(repeat);
    assert!(!state.should_quit);
}

#[test]
fn pure_fuzzy_favorites_recent_and_selection() {
    let mut state = state();
    state.sessions[1].favorite = true;
    state.sessions[1].last_connect_attempt_at = Some(Utc::now());
    assert_eq!(state.filtered_sessions()[0].id, "2");
    state.search = "bt".into();
    assert_eq!(state.filtered_sessions()[0].name, "beta");
    state.search.clear();
    state.handle_key(key(KeyCode::Char('F')));
    state.handle_key(key(KeyCode::Char('R')));
    assert_eq!(state.filtered_sessions().len(), 1);
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Action::None);
    state.handle_key(key(KeyCode::Down));
    assert_eq!(
        state.handle_key(key(KeyCode::Char('f'))),
        Action::Favorite {
            id: "2".into(),
            value: false
        }
    );
}

#[test]
fn pure_deletion_defaults_to_cancel_and_captures_id() {
    let mut state = state();
    state.handle_key(key(KeyCode::Char('x')));
    assert!(matches!(
        state.modal,
        Some(Modal::Delete {
            confirmed: false,
            ..
        })
    ));
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Action::None);
    state.handle_key(key(KeyCode::Char('x')));
    state.selected_session_id = Some("2".into());
    state.handle_key(key(KeyCode::Right));
    assert_eq!(
        state.handle_key(key(KeyCode::Enter)),
        Action::Delete {
            id: "1".into(),
            view: View::Sessions
        }
    );
    state.handle_key(key(KeyCode::Char('x')));
    state.handle_key(key(KeyCode::Esc));
    assert!(state.modal.is_none());
}

#[test]
fn pure_form_edit_cancel_and_save_event() {
    let mut state = state();
    state.handle_key(key(KeyCode::Char('a')));
    state.handle_key(key(KeyCode::Char('x')));
    state.handle_key(key(KeyCode::Tab));
    state.handle_key(key(KeyCode::Char('g')));
    state.handle_key(key(KeyCode::Backspace));
    state.handle_key(key(KeyCode::BackTab));
    if let Some(Modal::Form(form)) = &state.modal {
        assert_eq!(form.kind, FormKind::AddSession);
        assert_eq!(form.fields[0].1, "x");
        assert_eq!(form.focused, 0);
        assert_eq!(form.fields[1].1, "");
    } else {
        panic!("Missing native form");
    }
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)),
        Action::Save
    );
    state.handle_key(key(KeyCode::Esc));
    assert!(state.modal.is_none());
    assert!(!state.should_quit);
}

#[test]
fn native_session_forms_add_edit_clear_duplicate_favorite_delete() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().to_path_buf()).unwrap();
    let service = AppService::new(storage.clone());
    let mut app = TuiApp::new(service.clone(), storage.clone()).unwrap();
    apply(&mut app, KeyCode::Char('a'));
    assert!(save(&mut app).is_err());
    assert!(matches!(app.state.modal, Some(Modal::Form(ref form)) if form.error.is_some()));
    for (index, value) in [
        (0, "api"),
        (1, "prod"),
        (2, "127.0.0.1"),
        (3, "ubuntu"),
        (4, "2222"),
        (5, "~/.ssh/id_ed25519"),
        (6, "jump.example.com"),
        (7, "API server"),
        (8, "prod, web"),
    ] {
        form_field(&mut app, index, value);
    }
    save(&mut app).unwrap();
    let original = app.state.selected_session().unwrap();
    assert_eq!(original.tags, vec!["prod", "web"]);
    assert_eq!(original.port, 2222);
    apply(&mut app, KeyCode::Char('e'));
    for index in [1, 5, 6, 7, 8] {
        form_field(&mut app, index, "");
    }
    form_field(&mut app, 0, "api-edited");
    save(&mut app).unwrap();
    let edited = service.get(&original.id, None).unwrap();
    assert!(
        edited.group.is_none()
            && edited.identity_file.is_none()
            && edited.jump_host.is_none()
            && edited.description.is_none()
    );
    assert!(edited.tags.is_empty());
    apply(&mut app, KeyCode::Char('d'));
    form_field(&mut app, 0, "api-copy");
    save(&mut app).unwrap();
    let copy = app.state.selected_session().unwrap();
    assert_ne!(copy.id, original.id);
    assert_eq!(copy.name, "api-copy");
    apply(&mut app, KeyCode::Char('f'));
    assert!(service.get(&copy.id, None).unwrap().favorite);
    apply(&mut app, KeyCode::Char('x'));
    apply(&mut app, KeyCode::Enter);
    assert_eq!(storage.load_registry().unwrap().sessions.len(), 2);
    apply(&mut app, KeyCode::Char('x'));
    apply(&mut app, KeyCode::Right);
    apply(&mut app, KeyCode::Enter);
    assert_eq!(storage.load_registry().unwrap().sessions.len(), 1);
    assert_eq!(app.state.selected_session().unwrap().id, original.id);
}

#[test]
fn native_tunnel_forms_add_edit_dynamic_delete_and_run_action() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().to_path_buf()).unwrap();
    let mut registry = storage.load_registry().unwrap();
    registry.sessions.push(session("1", "alpha"));
    registry.next_id = 2;
    storage
        .update_registry(|stored| {
            *stored = registry;
            Ok(())
        })
        .unwrap();
    let service = AppService::new(storage.clone());
    let mut app = TuiApp::new(service.clone(), storage).unwrap();
    apply(&mut app, KeyCode::Char('t'));
    apply(&mut app, KeyCode::Char('a'));
    for (index, value) in [(0, "db"), (4, "5433"), (5, "localhost"), (6, "5432")] {
        form_field(&mut app, index, value);
    }
    save(&mut app).unwrap();
    let first = app.state.selected_tunnel().unwrap();
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::RunTunnel(first.id.clone())
    );
    apply(&mut app, KeyCode::Char('e'));
    form_field(&mut app, 0, "socks");
    form_field(&mut app, 2, "dynamic");
    assert!(save(&mut app).is_err());
    form_field(&mut app, 5, "");
    form_field(&mut app, 6, "");
    save(&mut app).unwrap();
    let edited = service.get_tunnel(&first.id).unwrap();
    assert_eq!(edited.kind, TunnelKind::Dynamic);
    assert!(edited.destination_host.is_none() && edited.destination_port.is_none());
    apply(&mut app, KeyCode::Char('x'));
    apply(&mut app, KeyCode::Right);
    apply(&mut app, KeyCode::Enter);
    assert!(app.state.tunnels.is_empty());
}

#[test]
fn pure_tunnel_selection_persists_across_search_and_reload() {
    let mut state = TuiState::new(
        vec![session("s", "alpha")],
        vec![tunnel("t1", "s"), tunnel("t2", "s")],
        22,
        true,
    );
    state.view = View::Tunnels;
    state.selected_tunnel_id = Some("t2".into());
    state.search = "unmatched".into();
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Action::None);
    state.search.clear();
    state.replace_items(
        state.sessions.clone(),
        vec![tunnel("t2", "s"), tunnel("t1", "s")],
    );
    assert_eq!(
        state.handle_key(key(KeyCode::Enter)),
        Action::RunTunnel("t2".into())
    );
}

#[test]
fn render_sessions_details_empty_and_no_color() {
    let mut state = state();
    state.sessions[0].favorite = true;
    let (text, terminal) = rendered(&state, 120, 30);
    assert!(text.contains("Clisshman"));
    assert!(text.contains("alpha"));
    assert!(text.contains("Last attempt: Never"));
    assert!(text.contains("Favorite: true"));
    assert!(terminal
        .backend()
        .buffer()
        .content
        .iter()
        .all(|cell| cell.fg == Color::Reset && cell.bg == Color::Reset));
    state.search = "unmatched".into();
    let (text, _) = rendered(&state, 100, 24);
    assert!(text.contains("No matches"));
    assert!(text.contains("No session selected"));
}

#[test]
fn render_native_forms_confirmation_tunnels_and_small_terminals() {
    let mut state = state();
    state.handle_key(key(KeyCode::Char('e')));
    let (text, _) = rendered(&state, 120, 30);
    assert!(text.contains("Edit session"));
    assert!(text.contains("Identity file"));
    assert!(text.contains("Tags (comma separated)"));
    assert!(text.contains("Ctrl+S: Save | Esc: Cancel"));
    state.handle_key(key(KeyCode::Esc));
    state.handle_key(key(KeyCode::Char('x')));
    let (text, _) = rendered(&state, 120, 30);
    assert!(text.contains("[Cancel]"));
    assert!(text.contains("This cannot be undone"));
    state.handle_key(key(KeyCode::Esc));
    state.tunnels.push(tunnel("t1", "1"));
    state.handle_key(key(KeyCode::Char('t')));
    let (text, _) = rendered(&state, 120, 30);
    assert!(text.contains("database"));
    assert!(text.contains("Kind: local"));
    state.handle_key(key(KeyCode::Char('e')));
    let (text, _) = rendered(&state, 120, 30);
    assert!(text.contains("Edit tunnel"));
    assert!(text.contains("Allow public bind (true/false)"));
    for (width, height) in [(1, 1), (10, 5), (40, 10)] {
        rendered(&state, width, height);
    }
}

#[test]
fn ping_runs_off_the_ui_thread_and_remains_responsive() {
    let dir = tempdir().unwrap();
    let storage = StorageManager::with_dir(dir.path().to_path_buf()).unwrap();
    let mut registry = storage.load_registry().unwrap();
    registry.sessions.push(session("1", "alpha"));
    registry.next_id = 2;
    storage
        .update_registry(|stored| {
            *stored = registry;
            Ok(())
        })
        .unwrap();
    let mut app = TuiApp::new(AppService::new(storage.clone()), storage).unwrap();
    apply(&mut app, KeyCode::Char('p'));
    assert!(app.state.status.starts_with("Pinging"));
    app.handle_key(key(KeyCode::Char('/')));
    app.handle_key(key(KeyCode::Char('a')));
    assert!(app.state.searching);
    assert_eq!(app.state.search, "a");
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
}
