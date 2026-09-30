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
        if lower.contains("uv")
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

pub fn port_in_use_message(kind: AppKind, port: u16) -> String {
    match kind {
        AppKind::Vite => format!("Port {port} is in use, trying another one..."),
        AppKind::Storybook => {
            format!("Port {port} is not available, using the next free port")
        }
        AppKind::Python => format!(
            "ERROR:    [Errno 48] Address already in use ({port}), trying the next port"
        ),
    }
}
