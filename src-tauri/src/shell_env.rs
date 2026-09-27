use crate::error::{Error, Result};

/// Cached login-shell environment (`$SHELL -l -i -c 'env -0'`).
/// GUI apps on macOS do not inherit PATH, so every child process uses this.
#[derive(Debug, Default)]
pub struct ShellEnv {
    loaded: bool,
}

impl ShellEnv {
    pub fn new() -> Self {
        Self { loaded: false }
    }

    pub async fn load(&mut self) -> Result<()> {
        let _ = self.loaded;
        Err(Error::NotImplemented("shell_env.load".into()))
    }

    pub async fn reload(&mut self) -> Result<()> {
        Err(Error::NotImplemented("shell_env.reload".into()))
    }
}
