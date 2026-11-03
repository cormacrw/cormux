import type { WireCommand } from './types'

/**
 * Commands with nothing worth faking: they'd need a real repo, engine or GitHub behind them, and
 * no e2e flow reaches them. Calling one rejects, so a flow that starts to depend on one fails
 * loudly instead of carrying on with a `null`. To use one, move it to a handler.
 */
export const UNSUPPORTED_COMMANDS = [
  'abort_workspace_git',
  'add_repo',
  'create_workspace_branch',
  'create_workspace_pull_request',
  'pause_thread',
  'pull_workspace',
  'push_workspace_branch',
  'rebase_workspace',
  'remove_repo',
  'rename_workspace',
  'resolve_all_approvals',
  'resume_thread',
  'retry_workspace_provisioning',
  'send_workspace_findings',
  'skip_workspace_provisioning_setup',
  'start_branch_review',
  'submit_workspace_review',
  'summarise_workspace',
  'sync_pull_requests',
  'test_repo_setup',
] as const satisfies readonly WireCommand[]

export type UnsupportedCommand = (typeof UNSUPPORTED_COMMANDS)[number]

export function isUnsupported(cmd: string) {
  return (UNSUPPORTED_COMMANDS as readonly string[]).includes(cmd)
}
