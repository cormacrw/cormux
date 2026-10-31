mod acp;
mod claude;
mod command;
mod detect;
mod events;
mod manager;
mod models;
pub mod prompt;

pub use detect::EngineStatus;
pub use events::{AgentEvent, EngineKind, MessageRole, StreamedEvent, ToolCallStatus, ToolKind};
pub use manager::{EngineRegistry, SpawnSpec};
pub use models::{ModelOption, ThreadModels, static_models};

pub use claude::claude_argv;

/// One line every engine is told at spawn, so it knows the thread renders media
/// it references in markdown — the same fix t3code made for the same problem.
/// Claude receives it as a system prompt; the ACP engines, which have no such
/// channel, get it appended to their first prompt (see `manager`). Absolute
/// paths work from anywhere: the core copies the file before serving it.
pub const MEDIA_INSTRUCTION: &str = "This conversation is rendered by Cormux, which shows Markdown images and linked videos inline in the thread. When a visual result helps the user — a screenshot, recording, plot or diagram, for example verifying a UI change in a browser — embed the file in your reply as plain Markdown: ![short description](/absolute/path/to/screenshot.png) for images, [short description](/absolute/path/to/recording.mp4) for videos (png, jpg, gif, webp; mp4, webm, mov). Use the absolute path the tool saved to, and do not embed every file you touch. Cormux also renders fenced mermaid blocks as diagrams, and a fenced chart block as a chart: JSON like {\"type\":\"bar\",\"title\":\"Bundle size (kB)\",\"labels\":[\"a\",\"b\"],\"series\":[{\"name\":\"Before\",\"values\":[1,2]}]}, where type is bar or line. Use them when a diagram or chart says it better than prose.";
