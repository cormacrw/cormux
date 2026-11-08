//! Whether a workspace's branch may open a pull request: one rule for the snapshot and for
//! Create PR.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::model::WorkspaceKind;
use crate::store::types::WorkspaceRow;

use super::PrRow;

/// The pull request already open from the workspace's branch, wherever it was opened: by the
/// workspace, any synced one whose head is the branch, or gh-stack (`stack_pr`, the one its
/// stack last read as having).
pub fn branch_pr(
    workspace: &WorkspaceRow,
    pull_requests: &[PrRow],
    stack_pr: Option<u32>,
) -> Option<i64> {
    workspace
        .pr_number
        .or_else(|| {
            pull_requests
                .iter()
                .find(|pr| {
                    pr.repo_id.as_deref() == Some(workspace.repo_id.as_str())
                        && pr.payload.head == workspace.branch
                })
                .map(|pr| pr.number)
        })
        .or(stack_pr.map(i64::from))
}

/// Why Create PR can't open one from the workspace's branch, or `None` when it can. The one
/// rule: the snapshot carries it for the header and menu, and Create PR enforces it.
pub fn create_pr_block(
    workspace: &WorkspaceRow,
    pull_requests: &[PrRow],
    stack_pr: Option<u32>,
) -> Option<&'static str> {
    if workspace.kind == WorkspaceKind::PrReview {
        return Some("A review workspace submits a review rather than opening a pull request.");
    }
    branch_pr(workspace, pull_requests, stack_pr).map(|_| "This branch already has a pull request.")
}

/// A workspace that can't open a pull request now, and why, for the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCreatePrBlock {
    pub workspace_id: String,
    pub reason: String,
}

/// `stack_pr` is what gh-stack last said each workspace's branch has.
pub fn create_pr_blocks(
    workspaces: &[WorkspaceRow],
    pull_requests: &[PrRow],
    stack_pr: impl Fn(&WorkspaceRow) -> Option<u32>,
) -> Vec<WorkspaceCreatePrBlock> {
    workspaces
        .iter()
        .filter_map(|workspace| {
            create_pr_block(workspace, pull_requests, stack_pr(workspace)).map(|reason| {
                WorkspaceCreatePrBlock {
                    workspace_id: workspace.id.clone(),
                    reason: reason.into(),
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::types::{OpenedPullRequest, PullRequestPayload};

    fn synced(repo_id: &str, number: i64, head: &str) -> PullRequestPayload {
        PullRequestPayload {
            author: "ana".into(),
            ..PullRequestPayload::opened_by_viewer(OpenedPullRequest {
                number,
                title: "Work".into(),
                head: head.into(),
                base: "main".into(),
                files: 1,
                html_url: format!("https://github.com/o/r/pull/{number}"),
                repo_full_name: "o/r".into(),
                repo_id: repo_id.into(),
            })
        }
    }

    #[test]
    fn a_branch_with_a_pr_from_anywhere_cant_open_another() {
        use crate::model::WorkspaceLifecycle;
        let lead = crate::store::fixtures::workspace("feat", WorkspaceLifecycle::Idle);
        let rows = |prs: &[PullRequestPayload]| prs.iter().map(PrRow::from).collect::<Vec<_>>();

        assert_eq!(create_pr_block(&lead, &[], None), None);
        let elsewhere = rows(&[synced("r2", 3, "feat"), synced("r1", 4, "other")]);
        assert_eq!(create_pr_block(&lead, &elsewhere, None), None);
        for (workspace, prs, stack_pr) in [
            (lead.clone().with_pr(7, None), vec![], None),
            (lead.clone(), rows(&[synced("r1", 8, "feat")]), None),
            (lead.clone(), vec![], Some(9)),
        ] {
            assert_eq!(
                create_pr_block(&workspace, &prs, stack_pr),
                Some("This branch already has a pull request.")
            );
        }
        let review = WorkspaceRow {
            kind: WorkspaceKind::PrReview,
            ..lead
        };
        assert!(create_pr_block(&review, &[], None).is_some());
    }
}
