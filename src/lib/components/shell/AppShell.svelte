<script lang="ts">
  import { app } from '$lib/state'
  import type { Snippet } from 'svelte'

  let { children }: { children: Snippet } = $props()
</script>

<div class="flex h-screen bg-background text-foreground">
  <aside class="flex w-60 shrink-0 flex-col border-r border-sidebar-border bg-sidebar">
    <div
      class="flex h-11 items-center pl-[76px] pr-3 text-sm font-medium"
      data-tauri-drag-region
    >
      Harness
    </div>
    <nav class="flex flex-1 flex-col gap-1 p-2">
      <button
        class="rounded-md px-2 py-1.5 text-left text-sm hover:bg-sidebar-accent"
        class:bg-sidebar-accent={app.view === 'homebase'}
        onclick={() => app.openHomebase()}
      >
        Homebase
      </button>
      <button
        class="rounded-md px-2 py-1.5 text-left text-sm hover:bg-sidebar-accent"
        class:bg-sidebar-accent={app.view === 'settings'}
        onclick={() => app.openSettings()}
      >
        Settings
      </button>
    </nav>
  </aside>
  <div class="flex min-w-0 flex-1 flex-col">
    {@render children()}
  </div>
</div>
