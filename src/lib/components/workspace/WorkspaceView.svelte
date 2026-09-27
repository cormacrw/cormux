<script lang="ts">
  import { onMount } from 'svelte'
  import { app, threads, workspaces } from '$lib/state'
  import { startStreamingSpike } from '$lib/ipc'

  const workspace = $derived(
    app.workspaceId ? workspaces.getById(app.workspaceId) : undefined,
  )
  const workspaceThreads = $derived(
    app.workspaceId ? threads.forWorkspace(app.workspaceId) : [],
  )

  const isLoadSpike = $derived(app.workspaceId === 'spike-load')
  const diffLines = Array.from(
    { length: 5000 },
    (_, i) => `${String(i + 1).padStart(4, ' ')}  const row = ${i}`,
  )

  let agentText = $state('')
  let ptyLines = $state<string[]>([])
  let fps = $state(0)
  let frames = 0
  let lastFpsAt = 0

  onMount(() => {
    if (!isLoadSpike) return

    let raf = 0
    lastFpsAt = performance.now()
    const tick = (now: number) => {
      frames += 1
      if (now - lastFpsAt >= 1000) {
        fps = frames
        frames = 0
        lastFpsAt = now
      }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)

    void startStreamingSpike(
      (chunk) => {
        agentText += chunk.text
      },
      (chunk) => {
        ptyLines = [...ptyLines.slice(-499), chunk.line]
      },
    )

    return () => cancelAnimationFrame(raf)
  })
</script>

<section class="flex flex-1 flex-col gap-4 p-6">
  <header>
    <h1 class="text-xl font-semibold tracking-tight">
      {workspace?.name ?? app.workspaceId ?? 'Workspace'}
    </h1>
    <p class="text-sm text-muted-foreground">
      {#if isLoadSpike}
        {fps} fps · {ptyLines.length} log lines · 5000-line diff
      {:else}
        {workspaceThreads.length} threads
      {/if}
    </p>
  </header>

  {#if isLoadSpike}
    <div class="grid min-h-0 flex-1 grid-cols-3 gap-3 text-xs">
      <pre
        class="overflow-auto rounded-md border border-border bg-muted/30 p-3 font-mono"
      >{agentText}</pre>
      <pre
        class="overflow-auto rounded-md border border-border bg-muted/30 p-3 font-mono"
      >{ptyLines.join('\n')}</pre>
      <pre
        class="overflow-auto rounded-md border border-border bg-muted/30 p-3 font-mono"
      >{diffLines.join('\n')}</pre>
    </div>
  {/if}
</section>
