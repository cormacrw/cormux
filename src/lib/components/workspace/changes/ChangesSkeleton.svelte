<script lang="ts">
  let { label = 'Loading changes…' }: { label?: string } = $props()

  // Stand-in for the file list while the Git tab mounts or its first diff loads.
  const FILES = [
    { name: '42%', lines: ['88%', '64%', '92%', '40%', '76%'] },
    { name: '30%', lines: ['70%', '96%', '52%'] },
    { name: '50%', lines: ['84%', '60%', '72%', '90%'] },
  ]
</script>

<div
  class="flex min-h-0 flex-1 flex-col overflow-hidden"
  role="status"
  data-od-id="changes-skeleton"
>
  <span class="sr-only">{label}</span>
  {#each FILES as file, index (index)}
    <div class="border-b border-border/60" aria-hidden="true">
      <div
        class="flex items-center gap-2 border-b border-border/40 px-3 py-2.5"
      >
        <span class="skeleton-bar size-4"></span>
        <span class="skeleton-bar h-3" style:width={file.name}></span>
        <span class="ml-auto skeleton-bar h-3 w-12"></span>
      </div>
      <div class="flex flex-col gap-2.5 px-3 py-3">
        {#each file.lines as width, line (line)}
          <span class="skeleton-bar h-2.5" style:width></span>
        {/each}
      </div>
    </div>
  {/each}
</div>
