use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use specta::Type;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

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
#[derive(Clone)]
pub struct Metrics {
    system: Arc<Mutex<System>>,
    latest: Arc<Mutex<Option<MemorySample>>>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new())),
            latest: Arc::new(Mutex::new(None)),
        }
    }

    /// The last reading, for readers that shouldn't pay for a walk of every process.
    pub fn latest(&self) -> Option<MemorySample> {
        self.latest.lock().ok()?.clone()
    }

    pub fn sample(&self, trees: &[(String, Vec<u32>)]) -> Result<MemorySample> {
        let mut system = self
            .system
            .lock()
            .map_err(|error| crate::error::Error::Metrics(error.to_string()))?;
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );

        let children = children_by_parent(&system);
        let total = tree_memory(&system, &children, std::process::id());

        let mut per_workspace = Vec::new();
        for (workspace_id, pids) in trees {
            let mut bytes = 0u64;
            for pid in pids {
                bytes = bytes.saturating_add(tree_memory(&system, &children, *pid));
            }
            per_workspace.push(WorkspaceMemory {
                workspace_id: workspace_id.clone(),
                bytes,
            });
        }

        let sample = MemorySample {
            total_bytes: total,
            per_workspace,
        };
        if let Ok(mut latest) = self.latest.lock() {
            *latest = Some(sample.clone());
        }
        Ok(sample)
    }
}

fn children_by_parent(system: &System) -> HashMap<u32, Vec<u32>> {
    let mut children = HashMap::<u32, Vec<u32>>::new();
    for (pid, process) in system.processes() {
        if let Some(parent) = process.parent() {
            children
                .entry(parent.as_u32())
                .or_default()
                .push(pid.as_u32());
        }
    }
    children
}

fn tree_memory(system: &System, children: &HashMap<u32, Vec<u32>>, root: u32) -> u64 {
    let mut total = 0u64;
    let mut seen = HashSet::from([root]);
    let mut stack = vec![root];
    while let Some(pid) = stack.pop() {
        if let Some(process) = system.process(Pid::from_u32(pid)) {
            total = total.saturating_add(process.memory());
        }
        for child in children.get(&pid).into_iter().flatten() {
            if seen.insert(*child) {
                stack.push(*child);
            }
        }
    }
    total
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
        assert_eq!(metrics.latest().unwrap().total_bytes, sample.total_bytes);
    }

    #[test]
    fn counts_a_child_in_its_parents_tree() {
        let mut child = std::process::Command::new("sleep")
            .arg("5")
            .spawn()
            .unwrap();
        let metrics = Metrics::new();
        let parent = std::process::id();
        let sample = metrics
            .sample(&[
                ("self".into(), vec![parent]),
                ("child".into(), vec![child.id()]),
            ])
            .unwrap();
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(sample.per_workspace[1].bytes > 0);
        assert!(sample.per_workspace[0].bytes > sample.per_workspace[1].bytes);
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
