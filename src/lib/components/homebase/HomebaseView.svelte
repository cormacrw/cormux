<script lang="ts">
  import { workspaces, app } from '$lib/state'

  function openLoadSpike() {
    app.openWorkspace('spike-load')
  }
</script>

<section class="flex flex-1 flex-col gap-4 p-6">
  <header>
    <h1 class="text-xl font-semibold tracking-tight">Homebase</h1>
    <p class="text-sm text-muted-foreground">
      {workspaces.workingCount} working · {workspaces.needsAttentionCount} need attention
    </p>
  </header>
  {#if workspaces.items.length === 0}
    <p class="text-sm text-muted-foreground">No workspaces yet.</p>
  {/if}
  <button
    class="w-fit rounded-md border border-border px-3 py-1.5 text-sm hover:bg-muted"
    onclick={openLoadSpike}
  >
    Streaming load spike
  </button>
</section>
