<script lang="ts">
  import {
    resolveLocalMedia,
    type ResolvedMedia,
  } from '$lib/thread/reply/resolve-media'
  import FileWarning from '@lucide/svelte/icons/file-warning'
  import MediaLightbox from './MediaLightbox.svelte'

  let {
    source,
    alt,
    workdir,
  }: {
    /** The local path or file URL the agent referenced. */
    source: string
    alt: string
    /** The thread's worktree, for relative references. */
    workdir?: string
  } = $props()

  let resolved = $state<ResolvedMedia | null | undefined>(undefined)
  let expanded = $state(false)

  $effect(() => {
    let live = true
    resolved = undefined
    void resolveLocalMedia(source, workdir).then((media) => {
      if (live) resolved = media
    })
    return () => {
      live = false
    }
  })

  const fileName = $derived(
    source
      .replace(/^file:\/\//, '')
      .split('/')
      .filter(Boolean)
      .pop() ?? source,
  )
  const caption = $derived(alt && alt !== fileName ? alt : '')
</script>

{#if resolved === undefined}
  <div
    class="my-2 flex h-24 max-w-[42rem] items-center justify-center rounded-lg border border-border bg-muted/40 text-xs text-muted-foreground"
    aria-label="Loading {fileName}"
  >
    {fileName}
  </div>
{:else if resolved === null}
  <p
    class="my-2 inline-flex max-w-[42rem] items-center gap-1.5 text-xs text-muted-foreground"
  >
    <FileWarning class="size-3.5 shrink-0" aria-hidden="true" />
    <span class="truncate" title={source}>{fileName} isn't available</span>
  </p>
{:else if resolved.kind === 'image'}
  <figure class="my-2 max-w-[42rem]">
    <button
      type="button"
      class="block w-full cursor-zoom-in overflow-hidden rounded-lg border border-border"
      aria-label="Enlarge {resolved.name}"
      onclick={() => (expanded = true)}
    >
      <img
        src={resolved.url}
        alt={alt || resolved.name}
        class="block max-h-96 w-full object-contain"
        loading="lazy"
      />
    </button>
    {#if caption}
      <figcaption class="mt-1 text-xs text-muted-foreground">
        {caption}
      </figcaption>
    {/if}
  </figure>
  {#if expanded}
    <MediaLightbox
      src={resolved.url}
      alt={alt || resolved.name}
      name={resolved.name}
      onClose={() => (expanded = false)}
    />
  {/if}
{:else}
  <figure class="my-2 max-w-[42rem]">
    <video
      src={resolved.url}
      controls
      preload="metadata"
      class="block max-h-96 w-full rounded-lg border border-border bg-black"
    >
      <track kind="captions" />
    </video>
    {#if caption}
      <figcaption class="mt-1 text-xs text-muted-foreground">
        {caption}
      </figcaption>
    {/if}
  </figure>
{/if}
