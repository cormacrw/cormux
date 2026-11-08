//! The Tauri adapter: which feature commands and events the webview gets, the event transport,
//! and channel subscriptions.

pub mod events;
pub mod subscriptions;

use tauri_specta::Builder;

use crate::{
    app, approvals, attachments, changes, clickup, create_pr, environment, findings, menu, repos,
    review, scratch, settings, skills, snapshot, stack, teardown, terminal, thread_media,
    transcript, turn, workspace_git, workspace_start,
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
    workspace_start::commands::rename_workspace,
    workspace_git::commands::switch_workspace_branch,
    workspace_git::commands::pull_workspace,
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
        .constant("MEDIA_EXTENSIONS", crate::media::extension_kinds())
        .constant("FINDINGS_BLOCK_TAG", crate::review::FINDINGS_BLOCK_TAG)
        .constant("OUTPUT_LINE_CAP", crate::process::OUTPUT_LINE_CAP)
        .constant("FIRST_CONVERSATION", crate::transcript::FIRST_CONVERSATION)
        .constant(
            "CONVERSATION_STEP_KINDS",
            crate::transcript::CONVERSATION_STEP_KINDS,
        )
        .constant(
            "WORKING_THREAD_STATUSES",
            crate::model::ThreadStatus::WORKING,
        )
        .constant(
            "MID_TURN_THREAD_STATUSES",
            crate::model::ThreadStatus::MID_TURN,
        )
        .events(tauri_specta::collect_events![
            events::StateChanged,
            events::MemorySampled,
            events::ToastRaised,
            events::NotificationRaised,
            menu::MenuChosen,
        ])
}

/// Writes `src/lib/ipc/bindings.ts`; the bindings test runs it, and CI checks the file is current.
#[cfg(test)]
fn export_bindings(builder: &Builder) {
    use specta_typescript::{BigIntExportBehavior, Typescript};
    const PATH: &str = "../src/lib/ipc/bindings.ts";
    builder
        .export(
            Typescript::new()
                .header("// @ts-nocheck - generated by tauri-specta; do not edit.\n")
                .bigint(BigIntExportBehavior::Number),
            PATH,
        )
        .expect("failed to export TypeScript IPC bindings");
    let exported = std::fs::read_to_string(PATH).expect("read exported bindings");
    std::fs::write(PATH, sort_constants(&exported)).expect("write sorted bindings");
}

/// tauri-specta keeps constants in a `HashMap`, so each export lists them in a different order
/// and CI's stale-bindings check would fail at random. Each constant is one line.
#[cfg(test)]
fn sort_constants(bindings: &str) -> String {
    const START: &str = "/** user-defined constants **/\n";
    const END: &str = "/** user-defined types **/";
    let (Some(start), Some(end)) = (bindings.find(START), bindings.find(END)) else {
        return bindings.to_string();
    };
    let block = start + START.len();
    let mut lines: Vec<&str> = bindings[block..end]
        .lines()
        .filter(|line| !line.is_empty())
        .collect();
    lines.sort_unstable();
    format!(
        "{}\n{}\n\n{}",
        &bindings[..block],
        lines.join("\n"),
        &bindings[end..]
    )
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

    #[test]
    fn export_typescript_bindings() {
        super::export_bindings(&super::builder());
    }

    #[test]
    fn constants_come_out_in_the_same_order_whatever_order_specta_wrote() {
        let written = |a: &str, b: &str| {
            format!(
                "head\n/** user-defined constants **/\n\n{a}\n{b}\n\n/** user-defined types **/\ntail\n"
            )
        };
        let one = super::sort_constants(&written("export const B = 2;", "export const A = 1;"));
        let other = super::sort_constants(&written("export const A = 1;", "export const B = 2;"));
        assert_eq!(one, other);
        assert_eq!(
            one,
            "head\n/** user-defined constants **/\n\nexport const A = 1;\nexport const B = 2;\n\n/** user-defined types **/\ntail\n"
        );
    }
}
