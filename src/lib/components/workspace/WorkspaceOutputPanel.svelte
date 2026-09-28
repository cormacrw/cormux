<script lang="ts">
  import { onMount } from 'svelte'
  import * as ScrollArea from '$lib/components/ui/scroll-area'
  import { subscribePty } from '$lib/ipc'
  import { stripAnsi } from '$lib/workspace/ansi-line'

  let {
    workspaceId,
    provisioning,
  }: { workspaceId: string; provisioning: boolean } = $props()

  let lines = $state<string[]>([])

  onMount(() => {
    void subscribePty(workspaceId, (chunk) => {
      const text = stripAnsi(chunk.line)
      if (!text.trim()) return
      lines = [...lines.slice(-499), text]
    })
  })
</script>

<ScrollArea.Root
  class="h-full min-h-[12rem] rounded-md border border-border bg-muted/20"
>
  <pre
    class="p-3 font-mono text-xs leading-relaxed text-foreground/90 whitespace-pre-wrap break-all"
    aria-live="polite">{lines.join('\n') ||
      (provisioning ? 'Waiting for setup output…' : 'No output yet')}</pre>
</ScrollArea.Root>
