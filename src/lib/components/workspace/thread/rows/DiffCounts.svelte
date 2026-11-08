<!-- A file's `+added −deleted`, as an edit in the thread shows it. -->
<script lang="ts">
  import type { DiffFile } from '$lib/ipc/bindings'

  let { file }: { file: Pick<DiffFile, 'added' | 'deleted'> } = $props()
</script>

<span class="text-success">+{file.added}</span>
<span class="text-destructive">−{file.deleted}</span>
