<script lang="ts">
  import { workspaces } from '$lib/state'
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
</section>
