<script lang="ts">
  import JsonTree from './JsonTree.svelte'

  let {
    value,
    name,
    depth = 0,
  }: { value: unknown; name?: string; depth?: number } = $props()

  const isBranch = $derived(typeof value === 'object' && value !== null)
  const entries = $derived(
    isBranch ? Object.entries(value as Record<string, unknown>) : [],
  )
  const brackets = $derived(Array.isArray(value) ? '[ ]' : '{ }')
  const tone = $derived(
    typeof value === 'string'
      ? 'text-(--syntax-string)'
      : typeof value === 'object'
        ? 'text-muted-foreground'
        : 'text-(--syntax-constant)',
  )
</script>

{#if isBranch && entries.length}
  <details open={depth < 2} class="json-branch">
    <summary class="cursor-pointer select-none">
      {#if name !== undefined}<span class="text-(--syntax-entity)">{name}</span
        >:{/if}
      <span class="text-muted-foreground">{brackets} {entries.length}</span>
    </summary>
    <div class="ml-3 border-l border-border pl-3">
      {#each entries as [key, child] (key)}
        <JsonTree value={child} name={key} depth={depth + 1} />
      {/each}
    </div>
  </details>
{:else}
  <div class="break-all">
    {#if name !== undefined}<span class="text-(--syntax-entity)">{name}</span
      >:{/if}
    <span class={tone}>{isBranch ? brackets : JSON.stringify(value)}</span>
  </div>
{/if}
