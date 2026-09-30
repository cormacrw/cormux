#[derive(Debug)]
pub enum EngineCommand {
    Prompt(String),
    Cancel,
    Shutdown,
}
