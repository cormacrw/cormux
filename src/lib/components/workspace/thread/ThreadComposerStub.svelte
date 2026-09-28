<script lang="ts">
  import type { Thread } from '$lib/state/threads.svelte'
  import { engineDisplayName, engineMark } from '$lib/sidebar/engine'

  let { thread }: { thread: Thread } = $props()

  const mark = $derived(engineMark(thread.engine))
  const engineName = $derived(engineDisplayName(thread.engine))
</script>

<div
  class="sticky bottom-0 z-10 border-t border-border/70 bg-background/95 px-1 py-3 backdrop-blur supports-[backdrop-filter]:bg-background/80"
  data-od-id="composer-stub"
>
  <div class="mx-auto flex max-w-[760px] flex-col gap-2">
    <div class="flex items-center gap-2 text-xs text-muted-foreground">
      <span
        class="flex size-5 items-center justify-center rounded bg-muted font-mono text-[9px] font-semibold"
        aria-hidden="true">{mark}</span
      >
      <span>{engineName} · {thread.role}</span>
    </div>
    <textarea
      class="min-h-[72px] w-full resize-none rounded-md border border-border bg-muted/20 px-3 py-2 text-sm text-muted-foreground outline-none"
      disabled
      placeholder="Message {thread.role}… (composer lands in COR-20)"
      aria-label="Message composer (coming soon)"
      data-od-id="composer-input"></textarea>
  </div>
</div>
