<script lang="ts">
  import type { Component } from 'svelte'
  import type { ReplyBlock } from '$lib/thread/reply/registry'
  import { BLOCK_COMPONENTS } from './blocks'

  let { block, workdir }: { block: ReplyBlock; workdir?: string } = $props()

  // The map pairs each kind with its component; TypeScript can't follow that through a union.
  const Block = $derived(
    BLOCK_COMPONENTS[block.kind] as Component<{
      data: ReplyBlock['data']
      workdir?: string
    }>,
  )
</script>

<Block data={block.data} {workdir} />
