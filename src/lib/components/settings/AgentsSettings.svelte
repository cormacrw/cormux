<script lang="ts">
  import { onMount } from 'svelte'
  import Check from '@lucide/svelte/icons/check'
  import { Switch } from '$lib/components/ui/switch'
  import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte'
  import SettingsRow from '$lib/components/settings/SettingsRow.svelte'
  import { commands } from '$lib/ipc'
  import type { EngineKind, EngineStatus } from '$lib/ipc/bindings'
  import {
    ENGINE_OPTIONS,
    engineInstallLabel,
  } from '$lib/new-workspace/engines'
  import { engineMark } from '$lib/sidebar/engine'
  import { settings } from '$lib/state'
  import { cn } from '$lib/utils'

  let engineStatuses = $state<EngineStatus[]>([])

  const statusByKind = $derived(
    new Map(engineStatuses.map((row) => [row.kind, row])),
  )

  onMount(() => {
    void commands.detectEngines().then((result) => {
      if (result.status === 'ok') engineStatuses = result.data
    })
  })

  function engineStatusLine(status: EngineStatus | undefined) {
    if (!status?.installed) return 'Not installed'
    const parts: string[] = []
    if (status.binary) parts.push(status.binary)
    if (status.version) parts.push(`v${status.version}`)
    if (status.signedIn === false) parts.push('Sign in required')
    if (status.signedIn === true) parts.push('Signed in')
    return parts.join(' · ') || 'Installed'
  }

  async function chooseEngine(kind: EngineKind) {
    await settings.setDefaultEngine(kind)
  }
</script>

<div
  class="set-group space-y-3"
  role="radiogroup"
  aria-labelledby="settings-engine-label"
  data-od-id="settings-default-engine"
>
  <p id="settings-engine-label" class="sr-only">Default engine</p>
  <SettingsPanel class="flex flex-col gap-2 p-3 sm:flex-row">
    {#each ENGINE_OPTIONS as option (option.kind)}
      {@const status = statusByKind.get(option.kind)}
      {@const selected = settings.defaultEngine === option.kind}
      {@const install = engineInstallLabel(status)}
      <label
        class={cn(
          'flex min-w-0 flex-1 cursor-pointer items-center gap-3 rounded-[18px_16px_18px_14px] px-3 py-3 outline-none focus-within:ring-2 focus-within:ring-ring',
          selected ? 'bg-custard text-cocoa' : 'hover:bg-background/40',
        )}
      >
        <input
          type="radio"
          name="default-engine"
          class="sr-only"
          value={option.kind}
          checked={selected}
          onchange={() => void chooseEngine(option.kind)}
        />
        <span
          class="grid size-9 shrink-0 place-items-center rounded-full font-display text-xs font-extrabold text-cocoa"
          style:background={selected ? 'var(--clay-leaf)' : 'var(--clay-pebble)'}
          aria-hidden="true"
        >
          {engineMark(option.kind)}
        </span>
        <span class="min-w-0 flex-1">
          <span class="flex flex-wrap items-center gap-2">
            <span class={cn('text-sm font-semibold', selected && 'text-cocoa')}
              >{option.label}</span
            >
            {#if install}
              <span class="text-[10px] text-destructive">{install}</span>
            {/if}
          </span>
          <span
            class={cn(
              'mt-0.5 block font-mono text-[10px]',
              selected ? 'text-cocoa/70' : 'text-muted-foreground',
            )}>{engineStatusLine(status)}</span
          >
        </span>
        {#if selected}
          <Check class="size-4 shrink-0 text-cocoa" aria-hidden="true" />
        {/if}
      </label>
    {/each}
  </SettingsPanel>
</div>

<div class="mt-3" data-od-id="settings-agent-permissions">
  <SettingsPanel>
    <SettingsRow
      title="Run everything"
      description="On, both engines use tools without asking. Off, they ask before every tool."
    >
      {#snippet control()}
        <Switch
          checked={settings.runEverything}
          onCheckedChange={(next) => void settings.setRunEverything(next)}
          aria-label="Run everything"
        />
      {/snippet}
    </SettingsRow>
  </SettingsPanel>
</div>
