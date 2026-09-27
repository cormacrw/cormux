<script lang="ts">
  import type { Snippet } from 'svelte'
  import Sidebar from './Sidebar.svelte'

  let { children }: { children: Snippet } = $props()
</script>

<div class="app-shell grid min-h-dvh bg-background text-foreground">
  <aside
    class="flex min-h-0 min-w-0 flex-col border-sidebar-border bg-sidebar max-[760px]:border-b max-[760px]:border-r-0 border-r"
  >
    <Sidebar />
  </aside>
  <main class="main-area flex min-h-0 min-w-0 flex-col">
    {@render children()}
  </main>
</div>
