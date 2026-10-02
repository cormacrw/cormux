<script lang="ts">
  import type { Snippet } from 'svelte'
  import Sidebar from './Sidebar.svelte'

  let { children }: { children: Snippet } = $props()
</script>

<div
  class="app-shell relative grid h-dvh min-h-0 overflow-hidden bg-background text-foreground"
>
  <aside
    class="flex min-h-0 min-w-0 flex-col border-r border-sidebar-border bg-sidebar"
  >
    <Sidebar />
  </aside>
  <main class="main-area flex min-h-0 min-w-0 flex-col">
    {@render children()}
  </main>
</div>
