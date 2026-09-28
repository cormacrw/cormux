import type { SettingRow } from '$lib/ipc/bindings'

export function reviewFindingsReady(
  settings: SettingRow[],
  workspaceId: string,
  threadRole: string,
  findingCount: number,
): boolean {
  if (findingCount === 0) return false
  if (threadRole !== 'Reviewer') return false
  const key = `review:${workspaceId}/status`
  return settings.some((row) => row.key === key && row.value === 'ready')
}
