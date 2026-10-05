<script lang="ts">
  import { RadioGroup } from 'bits-ui'
  import { setMode, userPrefersMode } from 'mode-watcher'
  import * as Select from '$lib/components/ui/select'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte'
  import SettingsRow from '$lib/components/settings/SettingsRow.svelte'
  import {
    CODE_FONT_SIZES,
    DEFAULT_CODE_FONT_SIZE,
    DEFAULT_UI_FONT_SIZE,
    FELTS,
    UI_FONT_SIZES,
    type FeltId,
  } from '$lib/state/appearance.svelte'
  import { appearance } from '$lib/state'

  function sizeLabel(size: number, fallback: number) {
    return size === fallback ? `${size}px (default)` : `${size}px`
  }
</script>

<SettingsPanel data-od-id="settings-appearance-group">
  <SettingsRow
    title="Felt colour"
    description="Re-dyes the background, sidebar, panels and primary buttons."
  >
    {#snippet control()}
      <RadioGroup.Root
        value={appearance.felt}
        onValueChange={(next) => appearance.setFelt(next as FeltId)}
        orientation="horizontal"
        aria-label="Felt colour"
        class="flex items-center gap-2.5"
      >
        {#each FELTS as felt (felt.id)}
          <RadioGroup.Item
            value={felt.id}
            aria-label={felt.label}
            title={felt.label}
            class="squish relative size-11 overflow-hidden rounded-full clay-sm outline-none data-[state=checked]:shadow-[0_0_0_3px_var(--card),0_0_0_6px_var(--cocoa)]"
            style="background: linear-gradient(90deg, {felt.sidebar} 0 42%, {felt.ground} 42% 100%)"
          >
            <span
              class="bead absolute right-1.5 bottom-1.5 size-3.5"
              style="background: {felt.primary}"
              aria-hidden="true"
            ></span>
          </RadioGroup.Item>
        {/each}
      </RadioGroup.Root>
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="Light or dark"
    description="Follow the system, or always use light or dark"
  >
    {#snippet control()}
      <ToggleGroup.Root
        type="single"
        size="sm"
        aria-label="Theme"
        bind:value={
          () => userPrefersMode.current,
          (next) => {
            // Clicking the pressed item deselects it; keep a mode selected.
            if (next) setMode(next as 'system' | 'light' | 'dark')
          }
        }
      >
        <ToggleGroup.Item value="system">System</ToggleGroup.Item>
        <ToggleGroup.Item value="light">Light</ToggleGroup.Item>
        <ToggleGroup.Item value="dark">Dark</ToggleGroup.Item>
      </ToggleGroup.Root>
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="UI font size"
    description="Text size across the app"
    controlId="settings-ui-font-size"
  >
    {#snippet control()}
      <Select.Root
        type="single"
        value={String(appearance.uiFontSize)}
        onValueChange={(next) => appearance.setUiFontSize(Number(next))}
      >
        <Select.Trigger id="settings-ui-font-size" class="w-[180px]">
          {sizeLabel(appearance.uiFontSize, DEFAULT_UI_FONT_SIZE)}
        </Select.Trigger>
        <Select.Content>
          {#each UI_FONT_SIZES as size (size)}
            <Select.Item value={String(size)}>
              {sizeLabel(size, DEFAULT_UI_FONT_SIZE)}
            </Select.Item>
          {/each}
        </Select.Content>
      </Select.Root>
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="Code font size"
    description="Diffs, command output and code blocks in threads"
    controlId="settings-code-font-size"
  >
    {#snippet control()}
      <Select.Root
        type="single"
        value={String(appearance.codeFontSize)}
        onValueChange={(next) => appearance.setCodeFontSize(Number(next))}
      >
        <Select.Trigger id="settings-code-font-size" class="w-[180px]">
          {sizeLabel(appearance.codeFontSize, DEFAULT_CODE_FONT_SIZE)}
        </Select.Trigger>
        <Select.Content>
          {#each CODE_FONT_SIZES as size (size)}
            <Select.Item value={String(size)}>
              {sizeLabel(size, DEFAULT_CODE_FONT_SIZE)}
            </Select.Item>
          {/each}
        </Select.Content>
      </Select.Root>
    {/snippet}
  </SettingsRow>
</SettingsPanel>
