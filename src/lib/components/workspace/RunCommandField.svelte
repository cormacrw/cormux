<script lang="ts">
  import { tick } from 'svelte'
  import Pencil from '@lucide/svelte/icons/pencil'
  import Undo2 from '@lucide/svelte/icons/undo-2'
  import { Button } from '$lib/components/ui/button'
  import { settings } from '$lib/state'
  import {
    overrideToSave,
    runCommandOverride,
    runCommandOverrideKey,
  } from '$lib/run-app/run-command'

  let {
    workspaceId,
    repoCommand,
    running = false,
  }: {
    workspaceId: string
    /** The repo's command, which every workspace uses unless it sets its own. */
    repoCommand: string
    running?: boolean
  } = $props()

  let editing = $state(false)
  let draft = $state('')
  let inputEl = $state<HTMLInputElement | null>(null)

  const override = $derived(runCommandOverride(settings.rows, workspaceId))
  const command = $derived(override || repoCommand)

  /** Opens the editor; the empty state uses it when there's no command at all. */
  export async function startEditing() {
    draft = command
    editing = true
    await tick()
    inputEl?.focus()
    inputEl?.select()
  }

  function save() {
    if (!editing) return
    editing = false
    const value = overrideToSave(draft, repoCommand)
    if (value !== override)
      void settings.persist(runCommandOverrideKey(workspaceId), value)
  }

  function reset() {
    void settings.persist(runCommandOverrideKey(workspaceId), '')
  }
</script>

{#if editing}
  <input
    bind:this={inputEl}
    bind:value={draft}
    type="text"
    spellcheck="false"
    autocomplete="off"
    aria-label="Run command for this workspace"
    placeholder={repoCommand || 'pnpm dev'}
    class="h-7 w-72 max-w-full rounded-md border border-input bg-background px-2 font-mono text-xs outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30"
    data-od-id="run-command-input"
    onkeydown={(event) => {
      if (event.key === 'Enter') {
        event.preventDefault()
        save()
      } else if (event.key === 'Escape') {
        event.preventDefault()
        event.stopPropagation()
        editing = false
      }
    }}
    onblur={save}
  />
{:else if command}
  <span class="flex min-w-0 items-center gap-1" data-od-id="run-command">
    <span
      class="max-w-[16rem] truncate font-mono text-xs text-muted-foreground"
      title={override
        ? `This workspace runs ${override} instead of the repo's ${repoCommand || 'command'}${running ? '. Restart to use it.' : '.'}`
        : command}>{command}</span
    >
    {#if override}
      <span
        class="shrink-0 rounded border border-border/60 px-1 text-[10px] text-muted-foreground"
        >this workspace</span
      >
    {/if}
    <Button
      variant="ghost"
      size="icon-xs"
      class="shrink-0 text-muted-foreground"
      aria-label="Edit the run command for this workspace"
      title="Edit the run command for this workspace"
      data-od-id="run-command-edit"
      onclick={() => void startEditing()}
    >
      <Pencil class="size-3" aria-hidden="true" />
    </Button>
    {#if override}
      <Button
        variant="ghost"
        size="icon-xs"
        class="shrink-0 text-muted-foreground"
        aria-label="Use the repo's run command"
        title={repoCommand
          ? `Use the repo's run command (${repoCommand})`
          : "Use the repo's run command"}
        data-od-id="run-command-reset"
        onclick={reset}
      >
        <Undo2 class="size-3" aria-hidden="true" />
      </Button>
    {/if}
  </span>
{/if}
