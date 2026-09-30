import type { WorkspaceLifecycle } from '$lib/ipc/bindings'

export type HomebaseFilter = 'all' | 'running' | 'idle'

export type HomebaseFilterCounts = {
  all: number
  running: number
  idle: number
}

export function isIdleLifecycle(lifecycle: WorkspaceLifecycle): boolean {
  return lifecycle === 'idle' || lifecycle === 'ready'
}

export function matchesHomebaseFilter(
  lifecycle: WorkspaceLifecycle,
  filter: HomebaseFilter,
): boolean {
  if (filter === 'all') return true
  if (filter === 'idle') return isIdleLifecycle(lifecycle)
  return !isIdleLifecycle(lifecycle)
}

export function homebaseFilterCounts(
  lifecycles: WorkspaceLifecycle[],
): HomebaseFilterCounts {
  let idle = 0
  for (const lifecycle of lifecycles) {
    if (isIdleLifecycle(lifecycle)) idle += 1
  }
  const all = lifecycles.length
  return { all, running: all - idle, idle }
}
