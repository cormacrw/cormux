use serde::{Deserialize, Serialize};
use specta::Type;

/// A workspace, space or folder offered by the Settings pickers.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClickupOption {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClickupUser {
    pub id: i64,
    pub username: String,
    pub initials: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClickupStatus {
    pub name: String,
    pub color: Option<String>,
    /// `open`, `custom`, `done` or `closed`. `custom` statuses are the ones between to do and done.
    pub kind: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClickupSprint {
    pub id: String,
    pub name: String,
    pub start_ms: Option<i64>,
    pub due_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClickupTask {
    pub id: String,
    pub custom_id: Option<String>,
    pub name: String,
    pub status: String,
    pub status_color: Option<String>,
    pub points: Option<f64>,
    pub assignees: Vec<ClickupUser>,
    pub priority: Option<String>,
    pub priority_color: Option<String>,
    pub parent: Option<String>,
    pub url: String,
}

/// Everything the sprint board renders: the current sprint, its lanes and its tasks.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClickupBoard {
    pub sprint: ClickupSprint,
    pub statuses: Vec<ClickupStatus>,
    pub tasks: Vec<ClickupTask>,
    /// The API key's owner, so Homebase can pick out their in-progress tasks.
    pub user_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClickupTag {
    pub name: String,
    pub fg: Option<String>,
    pub bg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClickupTaskDetail {
    pub task: ClickupTask,
    pub description: String,
    pub tags: Vec<ClickupTag>,
    pub creator: Option<ClickupUser>,
    pub list_name: Option<String>,
    pub due_ms: Option<i64>,
    pub created_ms: Option<i64>,
    pub updated_ms: Option<i64>,
}
