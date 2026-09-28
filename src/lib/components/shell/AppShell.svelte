<script lang="ts">
  import type { Snippet } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import Sidebar from './Sidebar.svelte'

  let { children }: { children: Snippet } = $props()

  function startWindowDrag(event: MouseEvent) {
    if (event.button !== 0) return
    void getCurrentWindow().startDragging()
  }
</script>

<div
  class="app-shell grid h-dvh min-h-0 overflow-hidden bg-background text-foreground"
>
  <aside
    class="flex min-h-0 min-w-0 flex-col border-r border-sidebar-border bg-sidebar"
  >
    <Sidebar />
  </aside>
  <div class="grid min-h-0 min-w-0 grid-rows-[2.5rem_minmax(0,1fr)]">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="titlebar h-10"
      data-tauri-drag-region
      onmousedown={startWindowDrag}
    ></div>
    <main class="main-area flex min-h-0 min-w-0 flex-col">
      {@render children()}
    </main>
  </div>
</div>
