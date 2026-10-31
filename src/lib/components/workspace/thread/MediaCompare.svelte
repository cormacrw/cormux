<script lang="ts">
  import {
    resolveLocalMedia,
    type ResolvedMedia,
  } from '$lib/thread/local-media'

  let {
    before,
    after,
    workdir,
  }: { before: string; after: string; workdir?: string } = $props()

  let images = $state<[ResolvedMedia, ResolvedMedia] | null>(null)
  let split = $state(50)

  $effect(() => {
    const sources = [before, after]
    let stale = false
    void Promise.all(
      sources.map((source) => resolveLocalMedia(source, workdir)),
    ).then(([first, second]) => {
      if (!stale && first && second) images = [first, second]
    })
    return () => {
      stale = true
    }
  })
</script>

{#if images}
  <figure class="reply-card reply-compare" data-od-id="reply-compare">
    <div class="relative overflow-hidden">
      <img src={images[1].url} alt="After" class="block w-full" />
      <img
        src={images[0].url}
        alt="Before"
        class="absolute inset-0 block size-full object-cover"
        style:clip-path="inset(0 {100 - split}% 0 0)"
      />
      <span class="reply-compare-rule" style:left="{split}%"></span>
      <span class="reply-compare-tag left-2">Before</span>
      <span class="reply-compare-tag right-2">After</span>
      <input
        type="range"
        min="0"
        max="100"
        bind:value={split}
        aria-label="Compare before and after"
        class="absolute inset-0 size-full cursor-ew-resize opacity-0"
      />
    </div>
  </figure>
{/if}
