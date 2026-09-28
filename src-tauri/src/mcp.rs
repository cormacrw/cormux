use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::approvals::ApprovalBroker;
use crate::error::{Error, Result};
use crate::app::WorkspaceAppService;
use crate::process::ProcessSupervisor;
use crate::store::Store;
use crate::store::types::FindingRow;

const SEVERITIES: &[&str] = &["blocking", "suggestion", "nit"];

/// In-process MCP server given to every engine (`report_finding`, `finish_review`, …).
///
/// Tools are invoked through [`CormuxMcp::call`] (and later over stdio via `rmcp`
/// once engine spawn wires MCP config). The dispatcher is the source of truth so
/// review/findings features can share one validation path.
#[derive(Clone)]
pub struct CormuxMcp {
    store: Store,
    process: ProcessSupervisor,
    approvals: Arc<ApprovalBroker>,
    apps: WorkspaceAppService,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpContext {
    pub workspace_id: String,
    pub thread_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolResult {
    pub ok: bool,
    pub payload: Value,
}

impl CormuxMcp {
    pub fn new(
        store: Store,
        process: ProcessSupervisor,
        approvals: Arc<ApprovalBroker>,
        apps: WorkspaceAppService,
    ) -> Self {
        Self {
            store,
            process,
            approvals,
            apps,
        }
    }

    pub fn tool_names() -> &'static [&'static str] {
        &[
            "report_finding",
            "finish_review",
            "mark_finding_fixed",
            "read_app_output",
            "restart_app",
        ]
    }

    pub fn call(&self, name: &str, args: &Value, ctx: &McpContext) -> Result<McpToolResult> {
        match name {
            "report_finding" => self.report_finding(args, ctx),
            "finish_review" => self.finish_review(ctx),
            "mark_finding_fixed" => self.mark_finding_fixed(args),
            "read_app_output" => self.read_app_output(ctx),
            "restart_app" => self.restart_app(args, ctx),
            other => Err(Error::Mcp(format!("unknown tool {other}"))),
        }
    }

    fn report_finding(&self, args: &Value, ctx: &McpContext) -> Result<McpToolResult> {
        let severity = args
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        if !SEVERITIES.contains(&severity.as_str()) {
            return Err(Error::Mcp(
                "severity must be blocking, suggestion, or nit".into(),
            ));
        }
        let title = required_string(args, "title")?;
        let explanation = required_string(args, "explanation")?;
        let file = args.get("file").and_then(Value::as_str).map(str::to_string);
        let line = args.get("line").and_then(Value::as_i64);
        let id = args
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        self.store.upsert_finding(&FindingRow {
            id: id.clone(),
            workspace_id: ctx.workspace_id.clone(),
            severity,
            title,
            file,
            line,
            explanation,
            status: "open".into(),
            commit_sha: None,
        })?;

        Ok(McpToolResult {
            ok: true,
            payload: json!({ "id": id }),
        })
    }

    fn finish_review(&self, ctx: &McpContext) -> Result<McpToolResult> {
        self.store
            .set_setting(&format!("review:{}/status", ctx.workspace_id), "ready")?;
        Ok(McpToolResult {
            ok: true,
            payload: json!({ "status": "ready" }),
        })
    }

    fn mark_finding_fixed(&self, args: &Value) -> Result<McpToolResult> {
        let id = required_string(args, "id")?;
        let commit = required_string(args, "commit")?;
        self.store.mark_finding_fixed(&id, &commit)?;
        Ok(McpToolResult {
            ok: true,
            payload: json!({ "id": id, "status": "fixed", "commit": commit }),
        })
    }

    fn read_app_output(&self, ctx: &McpContext) -> Result<McpToolResult> {
        let session_id = format!("{}-app", ctx.workspace_id);
        let output = format!(
            "{}\n{}",
            self.process.output_session(&session_id),
            self.process.output_session(&ctx.workspace_id)
        );
        Ok(McpToolResult {
            ok: true,
            payload: json!({ "output": output.trim() }),
        })
    }

    fn restart_app(&self, args: &Value, ctx: &McpContext) -> Result<McpToolResult> {
        let approved = args
            .get("approved")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !approved {
            let _ = self.approvals;
            return Err(Error::Mcp(
                "restart_app requires an approval before it can run".into(),
            ));
        }
        self.apps
            .restart_approved(&self.process, &self.store, &ctx.workspace_id)?;
        Ok(McpToolResult {
            ok: true,
            payload: json!({ "restarted": true }),
        })
    }
}

fn required_string(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| Error::Mcp(format!("{key} is required")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell_env::ShellEnv;
    use crate::store::types::{RepoRecord, WorkspaceRow};
    use tokio::sync::RwLock;

    fn mcp() -> (CormuxMcp, Store) {
        let store = Store::new();
        store.open_in_memory().unwrap();
        store
            .upsert_repo(&RepoRecord {
                id: "r1".into(),
                path: "/tmp/app".into(),
                name: "app".into(),
                default_branch: Some("main".into()),
                setup_commands: String::new(),
                run_command: Some("echo hi".into()),
            })
            .unwrap();
        store
            .upsert_workspace(&WorkspaceRow {
                id: "w1".into(),
                repo_id: "r1".into(),
                name: "Login".into(),
                branch: "feat".into(),
                worktree_path: "/tmp/wt".into(),
                status: "ready".into(),
                created_at: String::new(),
                summary: None,
                summary_at: None,
                summary_source: "Haiku 4.5".into(),
                kind: None,
                pr_number: None,
                modified_files: 0,
                archived_at: None,
            })
            .unwrap();
        let env = Arc::new(RwLock::new(ShellEnv::new()));
        let process = ProcessSupervisor::new(env);
        let approvals = Arc::new(ApprovalBroker::new());
        let apps = WorkspaceAppService::new();
        (CormuxMcp::new(store.clone(), process, approvals, apps), store)
    }

    fn ctx() -> McpContext {
        McpContext {
            workspace_id: "w1".into(),
            thread_id: "t1".into(),
        }
    }

    #[test]
    fn report_finding_validates_severity() {
        let (mcp, _) = mcp();
        let err = mcp
            .call(
                "report_finding",
                &json!({"severity":"loud","title":"x","explanation":"y"}),
                &ctx(),
            )
            .unwrap_err();
        assert!(err.to_string().contains("severity"));
    }

    #[test]
    fn report_finding_and_mark_fixed() {
        let (mcp, store) = mcp();
        let reported = mcp
            .call(
                "report_finding",
                &json!({
                    "severity": "blocking",
                    "title": "SQL injection",
                    "file": "api.ts",
                    "line": 12,
                    "explanation": "User input is concatenated."
                }),
                &ctx(),
            )
            .unwrap();
        let id = reported.payload["id"].as_str().unwrap().to_string();
        mcp.call(
            "mark_finding_fixed",
            &json!({ "id": id, "commit": "abc123" }),
            &ctx(),
        )
        .unwrap();
        let finding = store
            .snapshot()
            .unwrap()
            .findings
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(finding.status, "fixed");
        assert_eq!(finding.commit_sha.as_deref(), Some("abc123"));
    }

    #[test]
    fn finish_review_and_restart_gate() {
        let (mcp, store) = mcp();
        mcp.call("finish_review", &json!({}), &ctx()).unwrap();
        assert_eq!(
            store.get_setting("review:w1/status").unwrap().as_deref(),
            Some("ready")
        );
        let err = mcp.call("restart_app", &json!({}), &ctx()).unwrap_err();
        assert!(err.to_string().contains("approval"));
    }

    #[test]
    fn lists_required_tools() {
        assert!(CormuxMcp::tool_names().contains(&"report_finding"));
        assert!(CormuxMcp::tool_names().contains(&"restart_app"));
    }
}
