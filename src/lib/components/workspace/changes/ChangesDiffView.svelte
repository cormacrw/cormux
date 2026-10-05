<script lang="ts">
  import { DiffModeEnum, DiffView, SplitSide } from '@git-diff-view/svelte'
  import '@git-diff-view/svelte/styles/diff-view-pure.css'
  import { mode as theme } from 'mode-watcher'
  import type { DiffFile } from '$lib/ipc/bindings'
  import {
    diffComments,
    type DiffComment,
  } from '$lib/changes/diff-comments.svelte'
  import { toDiffViewData } from '$lib/changes/diff-view-data'
  import { appearance, workspaceUi } from '$lib/state'
  import { Button } from '$lib/components/ui/button'
  import { Textarea } from '$lib/components/ui/textarea'
  import { Kbd } from '$lib/components/ui/kbd'
  import CommandIcon from '@lucide/svelte/icons/command'
  import CornerDownLeftIcon from '@lucide/svelte/icons/corner-down-left'
  import MessageSquare from '@lucide/svelte/icons/message-square'
  import Trash2 from '@lucide/svelte/icons/trash-2'

  let {
    workspaceId,
    branch,
    file,
  }: {
    workspaceId: string
    branch: string | null
    file: DiffFile
  } = $props()

  const data = $derived(toDiffViewData(file))
  const mode = $derived(
    workspaceUi.diffMode === 'split'
      ? DiffModeEnum.Split
      : DiffModeEnum.Unified,
  )

  const extendData = $derived.by(() => {
    const oldFile: Record<string, { data: DiffComment[] }> = {}
    const newFile: Record<string, { data: DiffComment[] }> = {}
    for (const comment of diffComments.forFile(
      workspaceId,
      branch,
      file.path,
    )) {
      const bucket = comment.side === 'old' ? oldFile : newFile
      ;(bucket[comment.line] ??= { data: [] }).data.push(comment)
    }
    return { oldFile, newFile }
  })

  // Unsent text per line, so a diff refresh that closes the box doesn't lose it.
  const drafts: Record<string, string> = {}
  const draftKey = (side: SplitSide, line: number) =>
    `${branch}:${file.path}:${side}:${line}`

  function lineCode(
    diffFile: {
      getOldPlainLine: (n: number) => { value: string }
      getNewPlainLine: (n: number) => { value: string }
    },
    side: SplitSide,
    line: number,
  ) {
    const plain =
      side === SplitSide.old
        ? diffFile.getOldPlainLine(line)
        : diffFile.getNewPlainLine(line)
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
      branch,
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
    diffViewFontSize={appearance.codeFontSize}
  >
    {#snippet renderWidgetLine({ lineNumber, side, diffFile, onClose })}
      {@const key = draftKey(side, lineNumber)}
      <div class="diff-comment-row">
        <form
          class="diff-comment-card flex flex-col gap-2 p-2.5"
          onsubmit={(event) => {
            event.preventDefault()
            const body = new FormData(event.currentTarget).get('body')
            save(diffFile, side, lineNumber, String(body ?? ''), onClose)
          }}
        >
          <p
            class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"
          >
            <MessageSquare class="size-3.5 text-info" aria-hidden="true" />
            New comment on line {lineNumber}
          </p>
          <Textarea
            name="body"
            value={drafts[key] ?? ''}
            aria-label={`Comment on line ${lineNumber}`}
            placeholder="Leave a comment for the agent…"
            class="min-h-16 bg-background text-sm"
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
            <Button type="button" variant="ghost" size="sm" onclick={onClose}
              >Cancel<Kbd class="ml-1" aria-hidden="true">Esc</Kbd></Button
            >
            <Button type="submit" size="sm"
              >Add comment<Kbd class="ml-1 gap-0.5" aria-hidden="true"
                ><CommandIcon /><CornerDownLeftIcon /></Kbd
              ></Button
            >
          </div>
        </form>
      </div>
    {/snippet}

    {#snippet renderExtendLine({ data: comments })}
      <div class="diff-comment-row">
        <ul class="diff-comment-card divide-y divide-border/60">
          {#each comments as comment (comment.id)}
            <li class="group flex items-start gap-2.5 py-2 pr-1.5 pl-2.5">
              <span
                class="mt-px flex size-6 shrink-0 items-center justify-center rounded-full bg-info/15 text-info"
                aria-hidden="true"
              >
                <MessageSquare class="size-3.5" />
              </span>
              <div class="min-w-0 flex-1">
                <p class="text-xs text-muted-foreground">
                  <span class="font-medium text-foreground">You</span>
                  · line {comment.line}{comment.side === 'old'
                    ? ' (removed)'
                    : ''} · not sent yet
                </p>
                <p class="mt-0.5 text-sm whitespace-pre-wrap">
                  {comment.body}
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon-sm"
                class="text-muted-foreground opacity-0 group-focus-within:opacity-100 group-hover:opacity-100 hover:text-destructive"
                aria-label="Delete comment"
                title="Delete comment"
                onclick={() => diffComments.remove(workspaceId, comment.id)}
              >
                <Trash2 class="size-3.5" aria-hidden="true" />
              </Button>
            </li>
          {/each}
        </ul>
      </div>
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
  /* Comments sit in the diff as cards with an accent outline, so they read as notes rather than code. */
  .diff-comment-row {
    padding: 0.5rem 0.75rem;
    background: var(--background);
    font-family: var(--font-sans);
  }
  .diff-comment-card {
    max-width: 48rem;
    border: 1px solid color-mix(in oklch, var(--info) 35%, transparent);
    border-radius: var(--radius);
    background: var(--popover);
  }
  .changes-diff :global(.diff-line-syntax-raw),
  .changes-diff :global(.diff-line-content-raw) {
    font-family: var(--code-font-family);
  }
</style>
