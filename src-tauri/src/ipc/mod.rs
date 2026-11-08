//! The Tauri adapter: which feature commands and events the webview gets, the event transport,
//! and channel subscriptions.

pub mod events;
pub mod subscriptions;

use tauri_specta::Builder;

use crate::{
    app, approvals, attachments, changes, clickup, create_pr, environment, findings, menu, repos,
    review, scratch, settings, skills, snapshot, stack, teardown, terminal, thread_media,
    transcript, turn, workspace_git, workspace_start, workspaces,
};

/// The one list of commands: registers them with Tauri and names them for the allowlist test.
/// Each command's wire name is its function's name, so names stay unique across features.
macro_rules! commands {
    ($($($segment:ident)::+),* $(,)?) => {
        fn commands() -> tauri_specta::Commands<tauri::Wry> {
            tauri_specta::collect_commands![$($($segment)::+),*]
        }

        #[cfg(test)]
        const COMMAND_PATHS: &[&str] = &[$(stringify!($($segment)::+)),*];
    };
}

commands![
    snapshot::get_snapshot,
    environment::reload_environment,
    transcript::commands::get_thread_events,
    transcript::commands::get_thread_conversations,
    subscriptions::subscribe_agent_events,
    environment::detect_engines,
    turn::commands::send_thread_prompt,
    turn::commands::cancel_thread_turn,
    turn::commands::pause_thread,
    turn::commands::resume_thread,
    turn::commands::new_thread_session,
    turn::commands::resume_thread_session,
    turn::commands::thread_models,
    turn::commands::engine_models,
    turn::commands::set_thread_model,
    turn::commands::thread_efforts,
    turn::commands::engine_efforts,
    turn::commands::set_thread_effort,
    approvals::commands::resolve_approval,
    approvals::commands::answer_question,
    approvals::commands::resolve_all_approvals,
    subscriptions::subscribe_pty,
    subscriptions::subscribe_diffs,
    subscriptions::unsubscribe,
    changes::commands::refresh_workspace_diff,
    changes::commands::set_workspace_diff_target,
    changes::commands::discard_workspace_changes,
    clickup::commands::set_clickup_api_key,
    clickup::commands::clear_clickup_api_key,
    clickup::commands::clickup_workspaces,
    clickup::commands::clickup_spaces,
    clickup::commands::clickup_folders,
    clickup::commands::clickup_board,
    clickup::commands::clickup_task,
    clickup::commands::set_clickup_task_status,
    clickup::commands::set_clickup_task_points,
    repos::commands::list_repo_branches,
    repos::commands::list_repo_local_branches,
    repos::commands::delete_local_branch,
    workspace_start::commands::create_workspace,
    review::commands::create_review_workspace,
    review::commands::submit_workspace_review,
    review::commands::start_branch_review,
    skills::commands::list_thread_skills,
    skills::commands::list_repo_skills,
    workspace_start::commands::retry_workspace_provisioning,
    workspace_start::commands::skip_workspace_provisioning_setup,
    workspaces::rename_workspace,
    workspace_git::commands::switch_workspace_branch,
    workspace_git::commands::pull_workspace,
    workspace_git::commands::rebase_workspace,
    workspace_git::commands::push_workspace_branch,
    create_pr::commands::default_pr_prompt,
    create_pr::commands::draft_pr_why,
    create_pr::commands::create_workspace_pull_request,
    workspace_git::commands::abort_workspace_git,
    workspace_git::commands::create_workspace_branch,
    stack::commands::get_workspace_stack,
    stack::commands::add_stack_branch,
    stack::commands::push_stack,
    stack::commands::sync_stack,
    teardown::commands::get_teardown_preview,
    teardown::commands::teardown_workspace,
    terminal::open_workspace_terminal,
    app::commands::control_workspace_app,
    app::commands::set_workspace_run_command,
    repos::commands::set_repo_run_command,
    repos::commands::set_repo_single_instance,
    repos::commands::set_repo_setup_commands,
    repos::commands::set_repo_default_branch,
    repos::commands::get_repo_git,
    repos::commands::add_repo,
    repos::commands::remove_repo,
    repos::commands::test_repo_setup,
    settings::commands::get_settings,
    settings::commands::update_settings,
    findings::commands::send_workspace_findings,
    findings::commands::dismiss_workspace_findings,
    attachments::image_preview,
    thread_media::resolve_thread_media,
    scratch::end_scratch,
];

pub fn builder() -> Builder {
    Builder::<tauri::Wry>::new()
        .commands(commands())
        .constant("DEFAULT_SETTINGS", settings::Settings::default())
        .events(tauri_specta::collect_events![
            events::StateChanged,
            events::MemorySampled,
            events::ToastRaised,
            menu::MenuChosen,
        ])
}

/// Writes `src/lib/ipc/bindings.ts`; the bindings test runs it, and CI checks the file is current.
#[cfg(test)]
fn export_bindings(builder: &Builder) {
    use specta_typescript::{BigIntExportBehavior, Typescript};
    builder
        .export(
            Typescript::new()
                .header("// @ts-nocheck - generated by tauri-specta; do not edit.\n")
                .bigint(BigIntExportBehavior::Number),
            "../src/lib/ipc/bindings.ts",
        )
        .expect("failed to export TypeScript IPC bindings");
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    /// Tauri rejects any command missing from the allowlist, and the browser harness can't see
    /// it. An entry with no command behind it is dead permission surface.
    #[test]
    fn every_command_is_allowlisted() {
        let registered: BTreeSet<&str> = super::COMMAND_PATHS
            .iter()
            .map(|path| path.rsplit("::").next().unwrap().trim())
            .collect();
        assert_eq!(
            registered.len(),
            super::COMMAND_PATHS.len(),
            "two commands share a wire name"
        );
        let allowlisted: BTreeSet<&str> = include_str!("../../permissions/ipc.toml")
            .split("commands.allow = [")
            .nth(1)
            .expect("permissions/ipc.toml has a commands.allow list")
            .split(']')
            .next()
            .unwrap()
            .split('"')
            .skip(1)
            .step_by(2)
            .collect();
        let missing: Vec<_> = registered.difference(&allowlisted).collect();
        assert!(
            missing.is_empty(),
            "add to permissions/ipc.toml: {missing:?}"
        );
        let stale: Vec<_> = allowlisted.difference(&registered).collect();
        assert!(
            stale.is_empty(),
            "remove from permissions/ipc.toml: {stale:?}"
        );
    }

    /// Writes `src/lib/ipc/bindings.ts`; CI fails when the committed file differs afterwards.
    #[test]
    fn export_typescript_bindings() {
        super::export_bindings(&super::builder());
    }
}
