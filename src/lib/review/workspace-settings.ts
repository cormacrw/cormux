import type { SettingRow } from '$lib/ipc/bindings'

export function reviewReady(
  settings: SettingRow[],
  workspaceId: string,
): boolean {
  const key = `review:${workspaceId}/status`
  return settings.some((row) => row.key === key && row.value === 'ready')
}

export function reviewSubmitted(
  settings: SettingRow[],
  workspaceId: string,
): boolean {
  const key = `review:${workspaceId}/submitted`
  return settings.some((row) => row.key === key && row.value === 'true')
}
