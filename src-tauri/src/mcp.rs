use crate::error::{Error, Result};

/// In-process MCP server given to every engine (`report_finding`, `finish_review`, …).
#[derive(Debug, Default)]
pub struct HarnessMcp;

impl HarnessMcp {
    pub fn new() -> Self {
        Self
    }

    pub async fn start(&self) -> Result<()> {
        Err(Error::NotImplemented("mcp.start".into()))
    }
}
