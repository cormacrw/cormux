use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMemory {
    pub workspace_id: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MemorySample {
    pub total_bytes: u64,
    pub per_workspace: Vec<WorkspaceMemory>,
}

/// Sampled resident memory for Cormux, the webview, and child process trees.
pub struct Metrics {
    system: Mutex<System>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            system: Mutex::new(System::new()),
        }
    }

    pub fn sample(&self, trees: &[(String, Vec<u32>)]) -> Result<MemorySample> {
        let mut system = self
            .system
            .lock()
            .map_err(|error| crate::error::Error::Metrics(error.to_string()))?;
        system.refresh_processes(ProcessesToUpdate::All, true);

        let self_pid = std::process::id();
        let total = tree_memory(&system, self_pid);

        let mut per_workspace = Vec::new();
        for (workspace_id, pids) in trees {
            let mut bytes = 0u64;
            for pid in pids {
                bytes = bytes.saturating_add(tree_memory(&system, *pid));
            }
            per_workspace.push(WorkspaceMemory {
                workspace_id: workspace_id.clone(),
                bytes,
            });
        }

        Ok(MemorySample {
            total_bytes: total,
            per_workspace,
        })
    }
}

fn tree_memory(system: &System, root: u32) -> u64 {
    let mut total = 0u64;
    for (pid, process) in system.processes() {
        if in_tree(system, pid.as_u32(), root) {
            total = total.saturating_add(process.memory());
        }
    }
    total
}

fn in_tree(system: &System, pid: u32, root: u32) -> bool {
    if pid == root {
        return true;
    }
    let mut current = pid;
    let mut hops = 0;
    while hops < 64 {
        let Some(process) = system.process(Pid::from_u32(current)) else {
            return false;
        };
        let Some(parent) = process.parent() else {
            return false;
        };
        let parent = parent.as_u32();
        if parent == root {
            return true;
        }
        if parent == current {
            return false;
        }
        current = parent;
        hops += 1;
    }
    false
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_this_process() {
        let metrics = Metrics::new();
        let sample = metrics.sample(&[]).unwrap();
        assert!(sample.total_bytes > 0);
    }

    #[test]
    fn breaks_down_named_trees() {
        let metrics = Metrics::new();
        let pid = std::process::id();
        let sample = metrics.sample(&[("ws-1".into(), vec![pid])]).unwrap();
        assert_eq!(sample.per_workspace.len(), 1);
        assert!(sample.per_workspace[0].bytes > 0);
    }
}
