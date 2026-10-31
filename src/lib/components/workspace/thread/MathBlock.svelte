<script lang="ts">
  import { renderMath } from '$lib/thread/render-math'

  let { tex }: { tex: string } = $props()

  let html = $state<string | null>(null)

  $effect(() => {
    const source = tex
    let stale = false
    void renderMath(source, true).then((rendered) => {
      if (!stale) html = rendered
    })
    return () => {
      stale = true
    }
  })
</script>

<div class="reply-math" data-od-id="reply-math">
  {#if html === null}
    <code>{tex}</code>
  {:else}
    {@html html}
  {/if}
</div>
