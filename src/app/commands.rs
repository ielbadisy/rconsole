#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ask(String),
    R(String),
    RGlimpse(String),
    Codex(String),
    Help,
    Quit,
    Pwd,
    Context,
    Objects,
    ResetR,
    History,
    Clear,
}

impl Command {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ask(_) => "/ask",
            Self::R(_) => "/r",
            Self::RGlimpse(_) => "/r-glimpse",
            Self::Codex(_) => "/codex",
            Self::Help => "/help",
            Self::Quit => "/quit",
            Self::Pwd => "/pwd",
            Self::Context => "/context",
            Self::Objects => "/objects",
            Self::ResetR => "/reset-r",
            Self::History => "/history",
            Self::Clear => "/clear",
        }
    }
}
