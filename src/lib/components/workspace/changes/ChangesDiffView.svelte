<script lang="ts">
  import { DiffModeEnum, DiffView, SplitSide } from '@git-diff-view/svelte'
  import '@git-diff-view/svelte/styles/diff-view-pure.css'
  import { mode as theme } from 'mode-watcher'
  import type { DiffFile } from '$lib/ipc/bindings'
  import { diffComments, type DiffComment } from '$lib/changes/diff-comments.svelte'
  import { toDiffViewData } from '$lib/changes/diff-view-data'
  import { workspaceUi } from '$lib/state'
  import { Button } from '$lib/components/ui/button'
  import { Textarea } from '$lib/components/ui/textarea'
  import X from '@lucide/svelte/icons/x'

  let {
    workspaceId,
    file,
  }: {
    workspaceId: string
    file: DiffFile
  } = $props()

  const data = $derived(toDiffViewData(file))
  const mode = $derived(
    workspaceUi.diffMode === 'split' ? DiffModeEnum.Split : DiffModeEnum.Unified,
  )

  const extendData = $derived.by(() => {
    const oldFile: Record<string, { data: DiffComment[] }> = {}
    const newFile: Record<string, { data: DiffComment[] }> = {}
    for (const comment of diffComments.forFile(workspaceId, file.path)) {
      const bucket = comment.side === 'old' ? oldFile : newFile
      ;(bucket[comment.line] ??= { data: [] }).data.push(comment)
    }
    return { oldFile, newFile }
  })

  // Unsent text per line, so a diff refresh that closes the box doesn't lose it.
  const drafts: Record<string, string> = {}
  const draftKey = (side: SplitSide, line: number) => `${file.path}:${side}:${line}`

  function lineCode(diffFile: { getOldPlainLine: (n: number) => { value: string }; getNewPlainLine: (n: number) => { value: string } }, side: SplitSide, line: number) {
    const plain = side === SplitSide.old ? diffFile.getOldPlainLine(line) : diffFile.getNewPlainLine(line)
    return plain?.value?.replace(/\n$/, '') ?? ''
  }

  function save(
    diffFile: Parameters<typeof lineCode>[0],
    side: SplitSide,
    line: number,
    body: string,
    onClose: () => void,
  ) {
    if (!body.trim()) return
    diffComments.add(workspaceId, {
      path: file.path,
      side: side === SplitSide.old ? 'old' : 'new',
      line,
      code: lineCode(diffFile, side, line),
      body: body.trim(),
    })
    delete drafts[draftKey(side, line)]
    onClose()
  }
</script>

<div class="changes-diff">
  <DiffView
    {data}
    {extendData}
    diffViewMode={mode}
    diffViewTheme={theme.current === 'light' ? 'light' : 'dark'}
    diffViewHighlight
    diffViewAddWidget
    diffViewFontSize={12}
  >
    {#snippet renderWidgetLine({ lineNumber, side, diffFile, onClose })}
      {@const key = draftKey(side, lineNumber)}
      <form
        class="flex flex-col gap-2 border-y border-border/60 bg-card p-2 font-sans"
        onsubmit={(event) => {
          event.preventDefault()
          const body = new FormData(event.currentTarget).get('body')
          save(diffFile, side, lineNumber, String(body ?? ''), onClose)
        }}
      >
        <Textarea
          name="body"
          value={drafts[key] ?? ''}
          aria-label={`Comment on line ${lineNumber}`}
          placeholder="Leave a comment for the agent…"
          class="min-h-16 text-sm"
          autofocus
          oninput={(event) => (drafts[key] = event.currentTarget.value)}
          onkeydown={(event) => {
            if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
              event.preventDefault()
              event.currentTarget.form?.requestSubmit()
            } else if (event.key === 'Escape') {
              event.stopPropagation()
              onClose()
            }
          }}
        />
        <div class="flex justify-end gap-2">
          <Button type="button" variant="ghost" size="sm" onclick={onClose}>Cancel</Button>
          <Button type="submit" size="sm">Add comment</Button>
        </div>
      </form>
    {/snippet}

    {#snippet renderExtendLine({ data: comments })}
      <ul class="flex flex-col gap-1 border-y border-border/60 bg-card px-3 py-2 font-sans">
        {#each comments as comment (comment.id)}
          <li class="flex items-start gap-2 text-sm">
            <p class="min-w-0 flex-1 whitespace-pre-wrap">{comment.body}</p>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Delete comment"
              onclick={() => diffComments.remove(workspaceId, comment.id)}
            >
              <X class="size-3.5" aria-hidden="true" />
            </Button>
          </li>
        {/each}
      </ul>
    {/snippet}
  </DiffView>
</div>

<style>
  /* Match the app's surfaces instead of GitHub's palettes. */
  .changes-diff :global(.diff-tailwindcss-wrapper .diff-style-root) {
    --diff-plain-content--: var(--background);
    --diff-plain-lineNumber--: var(--background);
    --diff-expand-content--: var(--card);
    --diff-expand-lineNumber--: var(--card);
    --diff-empty-content--: var(--card);
    --diff-border--: var(--border);
    --diff-hunk-content--: var(--muted);
    --diff-hunk-lineNumber--: var(--muted);
    --diff-add-widget--: var(--primary);
    --diff-add-widget-color--: var(--primary-foreground);
  }
  .changes-diff :global(.diff-line-syntax-raw),
  .changes-diff :global(.diff-line-content-raw) {
    font-family: var(--font-mono);
  }
</style>
