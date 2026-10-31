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

/// Told to the engine at spawn: a thread starts off the default branch unless its prompt says otherwise.
pub const BRANCH_INSTRUCTION: &str = "This thread starts on a new branch cut from the repo's default branch, with a worktree of its own. If the user's first message names another branch to start from, or asks to pick up earlier work, run `git reset --hard <branch>` (`origin/<branch>` if it only exists there) before anything else, while this branch has no commits of its own, and say which branch you started from. Ignore this for later messages, and don't do it for a branch the user only mentions.";

/// One line the engine is told at spawn, so it knows the thread renders media
/// it references in markdown — the same fix t3code made for the same problem.
/// Claude receives it as a system prompt. Absolute paths work from anywhere: the
/// core copies the file before serving it.
pub const MEDIA_INSTRUCTION: &str = "This conversation is rendered by Cormux, which shows Markdown images and linked videos inline in the thread. Embed a file in your reply as plain Markdown: ![short description](/absolute/path/to/screenshot.png) for images, [short description](/absolute/path/to/recording.mp4) for videos (png, jpg, gif, webp; mp4, webm, mov). Use the absolute path the tool saved to. When the user asked for a change with a visible result (UI, layout, styling, rendered output), your closing reply must embed a screenshot or recording of the finished result without being asked. For other work, embed a screenshot, recording, plot or diagram only when it helps the user, and do not embed every file you touch. Cormux caches images by path, so every embedded image must be a new file at a path not yet embedded in this conversation, for example with a version or timestamp suffix. Never overwrite a file and embed it again; if a tool always writes to the same path, copy the result to a new unique filename and embed the copy. Cormux also renders fenced mermaid blocks as diagrams, and a fenced chart block as a chart: JSON like {\"type\":\"bar\",\"title\":\"Bundle size (kB)\",\"labels\":[\"a\",\"b\"],\"series\":[{\"name\":\"Before\",\"values\":[1,2]}]}, where type is bar or line. Use them when a diagram or chart says it better than prose.";
