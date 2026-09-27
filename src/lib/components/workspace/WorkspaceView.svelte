<script lang="ts">
  import { app, threads, workspaces } from '$lib/state'

  const workspace = $derived(
    app.workspaceId ? workspaces.getById(app.workspaceId) : undefined,
  )
  const workspaceThreads = $derived(
    app.workspaceId ? threads.forWorkspace(app.workspaceId) : [],
  )
</script>

<section class="flex flex-1 flex-col gap-4 p-6">
  <header>
    <h1 class="text-xl font-semibold tracking-tight">
      {workspace?.name ?? app.workspaceId ?? 'Workspace'}
    </h1>
    <p class="text-sm text-muted-foreground">
      {workspaceThreads.length} threads
    </p>
  </header>
</section>
