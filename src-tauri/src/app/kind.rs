#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppKind {
    Vite,
    Storybook,
    Python,
}

impl AppKind {
    pub fn detect(run_command: &str) -> Self {
        let lower = run_command.to_lowercase();
        if lower.contains("storybook") {
            return Self::Storybook;
        }
        if lower.split_whitespace().any(|word| word == "uv")
            || lower.contains("uvicorn")
            || lower.contains("fastapi")
            || lower.contains("python")
        {
            return Self::Python;
        }
        Self::Vite
    }

    pub fn base_port(self) -> u16 {
        match self {
            Self::Vite => 5173,
            Self::Storybook => 6006,
            Self::Python => 8000,
        }
    }
}
