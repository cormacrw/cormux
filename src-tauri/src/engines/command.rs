use super::prompt::Prompt;

#[derive(Debug)]
pub enum EngineCommand {
    Prompt(Prompt),
    Cancel,
    /// Switch the live session's model; `None` goes back to the engine's default.
    SetModel(Option<String>),
    /// Turn plan mode on or off for the live session. Only Claude has one.
    SetPlanMode(bool),
    Shutdown,
}
