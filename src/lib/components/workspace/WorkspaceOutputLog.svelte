<script lang="ts">
  import { ansiLineToHtml } from '$lib/workspace/ansi-html'
  import {
    OUTPUT_LINE_CLASS,
    classifyOutputLine,
  } from '$lib/workspace/output-log'

  let {
    lines,
    onScroll,
    logEl = $bindable(),
  }: {
    lines: string[]
    onScroll: () => void
    logEl?: HTMLDivElement
  } = $props()
</script>

<div
  bind:this={logEl}
  role="log"
  aria-label="App output"
  tabindex="0"
  aria-live="off"
  class="h-full min-h-[12rem] overflow-auto rounded-md border border-border bg-muted/20 p-3 font-mono text-[12.5px] leading-5"
  data-od-id="output-log"
  onscroll={onScroll}
>
  {#each lines as line, index (index)}
    {@const style = classifyOutputLine(line)}
    {#if style === 'blank'}
      <div class="h-5" aria-hidden="true"></div>
    {:else}
      <div class={`whitespace-pre-wrap break-all ${OUTPUT_LINE_CLASS[style]}`}>
        {@html ansiLineToHtml(line)}
      </div>
    {/if}
  {/each}
</div>
