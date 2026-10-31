import type { SettingRow } from '$lib/ipc/bindings'

export function reviewFindingsReady(
  settings: SettingRow[],
  workspaceId: string,
  findingCount: number,
): boolean {
  if (findingCount === 0) return false
  const key = `review:${workspaceId}/status`
  return settings.some((row) => row.key === key && row.value === 'ready')
}
