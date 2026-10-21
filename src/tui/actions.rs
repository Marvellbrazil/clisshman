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
    Transfer(String),
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
