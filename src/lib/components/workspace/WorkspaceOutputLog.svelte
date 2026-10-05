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

<!-- Focusable so the log can be scrolled from the keyboard. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  bind:this={logEl}
  role="log"
  aria-label="App output"
  tabindex="0"
  aria-live="off"
  class="min-h-[12rem] flex-1 overflow-auto bg-transparent p-4 font-mono text-(length:--code-font-size) leading-[1.6] text-[#fff1de]"
  data-od-id="output-log"
  onscroll={onScroll}
>
  {#each lines as line, index (index)}
    {@const style = classifyOutputLine(line)}
    {#if style === 'blank'}
      <div class="h-5" aria-hidden="true"></div>
    {:else}
      <div class={`whitespace-pre-wrap break-all ${OUTPUT_LINE_CLASS[style]}`}>
        <!-- eslint-disable-next-line svelte/no-at-html-tags -- ansiLineToHtml escapes the line first -->
        {@html ansiLineToHtml(line)}
      </div>
    {/if}
  {/each}
</div>
