//! ClickUp sprint board. The API key lives in the Keychain and never reaches the webview.

mod sprint;
pub mod types;

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::AUTHORIZATION;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::error::{Error, Result};
use sprint::{current_sprint, parse_ms};
use types::{
    ClickupBoard, ClickupOption, ClickupSprint, ClickupStatus, ClickupTag, ClickupTask,
    ClickupTaskDetail, ClickupUser,
};

const KEYRING_SERVICE: &str = "cormux";
const KEYRING_USER: &str = "clickup-api-key";
const API: &str = "https://api.clickup.com/api/v2";
/// Get Tasks returns 100 a page; a sprint past 1,000 tasks isn't a sprint.
const MAX_PAGES: u32 = 10;

fn read_stored_key() -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?;
    entry.get_password().ok().filter(|key| !key.is_empty())
}

fn keyring_error(error: keyring::Error) -> Error {
    Error::Clickup(error.to_string())
}

#[derive(Clone)]
pub struct ClickupClient {
    http: reqwest::Client,
    /// Outer `None` until the Keychain is first read, so every snapshot doesn't hit it.
    key: Arc<Mutex<Option<Option<String>>>>,
    user_id: Arc<Mutex<Option<i64>>>,
}

impl Default for ClickupClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ClickupClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .unwrap_or_default(),
            key: Arc::new(Mutex::new(None)),
            user_id: Arc::new(Mutex::new(None)),
        }
    }

    fn key(&self) -> Option<String> {
        self.key
            .lock()
            .unwrap()
            .get_or_insert_with(read_stored_key)
            .clone()
    }

    pub fn is_configured(&self) -> bool {
        self.key().is_some()
    }

    /// Checks the key against ClickUp before storing it, so a typo fails here and not on the board.
    pub async fn save_key(&self, key: &str) -> Result<()> {
        let key = key.trim();
        if key.is_empty() {
            return Err(Error::Clickup("API key must not be empty".into()));
        }
        let user = self.fetch_user(key).await?;
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .and_then(|entry| entry.set_password(key))
            .map_err(keyring_error)?;
        *self.key.lock().unwrap() = Some(Some(key.to_string()));
        *self.user_id.lock().unwrap() = Some(user);
        Ok(())
    }

    pub fn clear_key(&self) -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(keyring_error)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(error) => return Err(keyring_error(error)),
        }
        *self.key.lock().unwrap() = Some(None);
        *self.user_id.lock().unwrap() = None;
        Ok(())
    }

    fn require_key(&self) -> Result<String> {
        self.key()
            .ok_or_else(|| Error::Clickup("add a ClickUp API key in Settings".into()))
    }

    async fn send<T: DeserializeOwned>(
        &self,
        key: &str,
        request: reqwest::RequestBuilder,
    ) -> Result<T> {
        let response = request
            .header(AUTHORIZATION, key)
            .send()
            .await
            .map_err(|error| Error::Clickup(format!("couldn't reach ClickUp: {error}")))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| Error::Clickup(error.to_string()))?;
        if !status.is_success() {
            return Err(Error::Clickup(api_error_message(status.as_u16(), &body)));
        }
        serde_json::from_str(&body)
            .map_err(|error| Error::Clickup(format!("unexpected ClickUp response: {error}")))
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let key = self.require_key()?;
        let request = self.http.get(format!("{API}{path}")).query(query);
        self.send(&key, request).await
    }

    async fn fetch_user(&self, key: &str) -> Result<i64> {
        let response: UserResponse = self.send(key, self.http.get(format!("{API}/user"))).await?;
        Ok(response.user.id)
    }

    async fn user_id(&self) -> Result<i64> {
        if let Some(id) = *self.user_id.lock().unwrap() {
            return Ok(id);
        }
        let id = self.fetch_user(&self.require_key()?).await?;
        *self.user_id.lock().unwrap() = Some(id);
        Ok(id)
    }

    pub async fn workspaces(&self) -> Result<Vec<ClickupOption>> {
        let response: TeamsResponse = self.get("/team", &[]).await?;
        Ok(response.teams.into_iter().map(Named::into_option).collect())
    }

    pub async fn spaces(&self, workspace_id: &str) -> Result<Vec<ClickupOption>> {
        let response: SpacesResponse = self
            .get(
                &format!("/team/{workspace_id}/space"),
                &[("archived", "false")],
            )
            .await?;
        Ok(response
            .spaces
            .into_iter()
            .map(Named::into_option)
            .collect())
    }

    pub async fn folders(&self, space_id: &str) -> Result<Vec<ClickupOption>> {
        let response: FoldersResponse = self
            .get(
                &format!("/space/{space_id}/folder"),
                &[("archived", "false")],
            )
            .await?;
        Ok(response
            .folders
            .into_iter()
            .map(Named::into_option)
            .collect())
    }

    pub async fn board(&self, folder_id: &str) -> Result<ClickupBoard> {
        let response: ListsResponse = self
            .get(
                &format!("/folder/{folder_id}/list"),
                &[("archived", "false")],
            )
            .await?;
        let sprints: Vec<ClickupSprint> = response.lists.into_iter().map(map_sprint).collect();
        let sprint = current_sprint(&sprints, now_ms())
            .cloned()
            .ok_or_else(|| Error::Clickup("the sprint folder has no sprints".into()))?;

        let list: ListResponse = self.get(&format!("/list/{}", sprint.id), &[]).await?;
        let tasks = self.sprint_tasks(&sprint.id).await?;
        let mut statuses: Vec<ClickupStatus> = list.statuses.into_iter().map(map_status).collect();
        // A list that inherits its statuses can come back without them; the tasks still carry theirs.
        for task in &tasks {
            if !statuses
                .iter()
                .any(|status| status.name.eq_ignore_ascii_case(&task.status.status))
            {
                statuses.push(map_status(task.status.clone()));
            }
        }
        statuses.sort_by_key(|status| status.order);

        Ok(ClickupBoard {
            sprint,
            statuses,
            tasks: tasks.into_iter().map(map_task).collect(),
            user_id: self.user_id().await?,
        })
    }

    async fn sprint_tasks(&self, list_id: &str) -> Result<Vec<TaskNode>> {
        let mut tasks = Vec::new();
        for page in 0..MAX_PAGES {
            let page = page.to_string();
            // Sprints usually hold tasks whose home list is elsewhere, which only `include_timl` returns.
            let response: TasksResponse = self
                .get(
                    &format!("/list/{list_id}/task"),
                    &[
                        ("page", &page),
                        ("include_closed", "true"),
                        ("include_timl", "true"),
                        ("subtasks", "true"),
                    ],
                )
                .await?;
            tasks.extend(response.tasks);
            if response.last_page.unwrap_or(true) {
                break;
            }
        }
        Ok(tasks)
    }

    pub async fn task(&self, task_id: &str) -> Result<ClickupTaskDetail> {
        let node: TaskNode = self
            .get(
                &format!("/task/{task_id}"),
                &[("include_markdown_description", "true")],
            )
            .await?;
        Ok(map_task_detail(node))
    }

    async fn update_task(&self, task_id: &str, body: Value) -> Result<()> {
        let key = self.require_key()?;
        let request = self.http.put(format!("{API}/task/{task_id}")).json(&body);
        let _: Value = self.send(&key, request).await?;
        Ok(())
    }

    pub async fn set_status(&self, task_id: &str, status: &str) -> Result<()> {
        self.update_task(task_id, json!({ "status": status })).await
    }

    pub async fn set_points(&self, task_id: &str, points: f64) -> Result<()> {
        if !points.is_finite() || points < 0.0 {
            return Err(Error::Clickup("sprint points must be zero or more".into()));
        }
        self.update_task(task_id, json!({ "points": points })).await
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or_default()
}

/// ClickUp errors look like `{"err":"Token invalid","ECODE":"OAUTH_025"}`.
fn api_error_message(status: u16, body: &str) -> String {
    if status == 401 {
        return "ClickUp rejected the API key".into();
    }
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.get("err").and_then(Value::as_str).map(str::to_string));
    match detail {
        Some(detail) => format!("ClickUp: {detail}"),
        None => format!("ClickUp returned HTTP {status}"),
    }
}

/// `orderindex` arrives as a number on some endpoints and a string on others.
fn flexible_i64<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<i64, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::Number(number) => number.as_i64().unwrap_or_default(),
        Value::String(text) => text.parse().unwrap_or_default(),
        _ => 0,
    })
}

/// Points come back as a number, or as a numeric string from older workspaces.
fn flexible_f64<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<f64>, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    })
}

#[derive(Deserialize)]
struct UserResponse {
    user: UserNode,
}

#[derive(Deserialize)]
struct Named {
    id: String,
    name: String,
}

impl Named {
    fn into_option(self) -> ClickupOption {
        ClickupOption {
            id: self.id,
            name: self.name,
        }
    }
}

#[derive(Deserialize)]
struct TeamsResponse {
    teams: Vec<Named>,
}

#[derive(Deserialize)]
struct SpacesResponse {
    spaces: Vec<Named>,
}

#[derive(Deserialize)]
struct FoldersResponse {
    folders: Vec<Named>,
}

#[derive(Deserialize)]
struct ListsResponse {
    lists: Vec<ListNode>,
}

#[derive(Deserialize)]
struct ListNode {
    id: String,
    name: String,
    start_date: Option<String>,
    due_date: Option<String>,
}

#[derive(Deserialize)]
struct ListResponse {
    #[serde(default)]
    statuses: Vec<StatusNode>,
}

#[derive(Deserialize, Clone)]
struct StatusNode {
    status: String,
    color: Option<String>,
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default, deserialize_with = "flexible_i64")]
    orderindex: i64,
}

#[derive(Deserialize)]
struct TasksResponse {
    tasks: Vec<TaskNode>,
    last_page: Option<bool>,
}

#[derive(Deserialize)]
struct UserNode {
    id: i64,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    initials: Option<String>,
    color: Option<String>,
}

#[derive(Deserialize)]
struct PriorityNode {
    priority: String,
    color: Option<String>,
}

#[derive(Deserialize)]
struct TagNode {
    name: String,
    tag_fg: Option<String>,
    tag_bg: Option<String>,
}

#[derive(Deserialize)]
struct ListRef {
    name: Option<String>,
}

#[derive(Deserialize)]
struct TaskNode {
    id: String,
    custom_id: Option<String>,
    name: String,
    status: StatusNode,
    #[serde(default, deserialize_with = "flexible_f64")]
    points: Option<f64>,
    #[serde(default)]
    assignees: Vec<UserNode>,
    priority: Option<PriorityNode>,
    parent: Option<String>,
    url: String,
    #[serde(default)]
    markdown_description: Option<String>,
    #[serde(default)]
    text_content: Option<String>,
    #[serde(default)]
    tags: Vec<TagNode>,
    creator: Option<UserNode>,
    list: Option<ListRef>,
    due_date: Option<String>,
    date_created: Option<String>,
    date_updated: Option<String>,
}

fn map_sprint(list: ListNode) -> ClickupSprint {
    ClickupSprint {
        id: list.id,
        name: list.name,
        start_ms: parse_ms(list.start_date.as_deref()),
        due_ms: parse_ms(list.due_date.as_deref()),
    }
}

fn map_status(status: StatusNode) -> ClickupStatus {
    ClickupStatus {
        name: status.status,
        color: status.color,
        kind: status.kind,
        order: status.orderindex,
    }
}

fn map_user(user: UserNode) -> ClickupUser {
    let username = user.username.unwrap_or_default();
    let initials = user.initials.unwrap_or_else(|| {
        username
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase()
    });
    ClickupUser {
        id: user.id,
        username,
        initials,
        color: user.color,
    }
}

fn map_task(task: TaskNode) -> ClickupTask {
    let (priority, priority_color) = match task.priority {
        Some(priority) => (Some(priority.priority), priority.color),
        None => (None, None),
    };
    ClickupTask {
        id: task.id,
        custom_id: task.custom_id,
        name: task.name,
        status: task.status.status,
        status_color: task.status.color,
        points: task.points,
        assignees: task.assignees.into_iter().map(map_user).collect(),
        priority,
        priority_color,
        parent: task.parent,
        url: task.url,
    }
}

fn map_task_detail(mut node: TaskNode) -> ClickupTaskDetail {
    let description = node
        .markdown_description
        .take()
        .filter(|text| !text.trim().is_empty())
        .or_else(|| node.text_content.take())
        .unwrap_or_default();
    let tags = std::mem::take(&mut node.tags)
        .into_iter()
        .map(|tag| ClickupTag {
            name: tag.name,
            fg: tag.tag_fg,
            bg: tag.tag_bg,
        })
        .collect();
    let creator = node.creator.take().map(map_user);
    let list_name = node.list.take().and_then(|list| list.name);
    let due_ms = parse_ms(node.due_date.as_deref());
    let created_ms = parse_ms(node.date_created.as_deref());
    let updated_ms = parse_ms(node.date_updated.as_deref());
    ClickupTaskDetail {
        task: map_task(node),
        description,
        tags,
        creator,
        list_name,
        due_ms,
        created_ms,
        updated_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_a_task_from_the_api_shape() {
        let raw = r##"{
            "id": "86abc", "custom_id": "ENG-142", "name": "Fix login",
            "status": {"status": "in progress", "color": "#4194f6", "type": "custom", "orderindex": 1},
            "points": null,
            "assignees": [{"id": 7, "username": "Sam Lee", "color": "#7b68ee", "initials": "SL"}],
            "priority": {"priority": "high", "color": "#ffcc00"},
            "parent": null, "url": "https://app.clickup.com/t/86abc",
            "markdown_description": "", "text_content": "Plain text",
            "tags": [{"name": "auth", "tag_fg": "#fff", "tag_bg": "#000"}],
            "list": {"id": "1", "name": "Sprint 12"},
            "due_date": "1727740800000", "date_created": "1727000000000", "date_updated": null
        }"##;
        let detail = map_task_detail(serde_json::from_str(raw).unwrap());
        assert_eq!(detail.task.custom_id.as_deref(), Some("ENG-142"));
        assert_eq!(detail.task.status, "in progress");
        assert_eq!(detail.task.points, None);
        assert_eq!(detail.task.assignees[0].initials, "SL");
        assert_eq!(detail.description, "Plain text");
        assert_eq!(detail.list_name.as_deref(), Some("Sprint 12"));
        assert_eq!(detail.due_ms, Some(1_727_740_800_000));
    }

    #[test]
    fn reads_points_and_order_in_either_encoding() {
        let raw = r#"{"id":"1","name":"a","url":"u","points":"3",
            "status":{"status":"to do","color":null,"type":"open","orderindex":"0"}}"#;
        let task: TaskNode = serde_json::from_str(raw).unwrap();
        assert_eq!(task.points, Some(3.0));
        assert_eq!(task.status.orderindex, 0);
    }

    #[test]
    fn explains_api_errors() {
        assert_eq!(api_error_message(401, ""), "ClickUp rejected the API key");
        assert_eq!(
            api_error_message(400, r#"{"err":"Status does not exist","ECODE":"ITEM_114"}"#),
            "ClickUp: Status does not exist"
        );
        assert_eq!(api_error_message(500, "oops"), "ClickUp returned HTTP 500");
    }

    #[test]
    fn keyring_constants_are_stable() {
        assert_eq!(KEYRING_SERVICE, "cormux");
        assert_eq!(KEYRING_USER, "clickup-api-key");
    }
}
