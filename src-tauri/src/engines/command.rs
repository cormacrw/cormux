#[derive(Debug)]
pub enum EngineCommand {
    Prompt(String),
    Cancel,
    /// Switch the live session's model; `None` goes back to the engine's default.
    SetModel(Option<String>),
    /// Turn plan mode on or off for the live session. Only Claude has one.
    SetPlanMode(bool),
    Shutdown,
}
