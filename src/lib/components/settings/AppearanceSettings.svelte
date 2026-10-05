<script lang="ts">
  import { RadioGroup } from 'bits-ui'
  import CheckIcon from '@lucide/svelte/icons/check'
  import { setMode, userPrefersMode } from 'mode-watcher'
  import * as Select from '$lib/components/ui/select'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte'
  import SettingsRow from '$lib/components/settings/SettingsRow.svelte'
  import {
    ACCENTS,
    CODE_FONTS,
    CODE_FONT_SIZES,
    DEFAULT_CODE_FONT,
    DEFAULT_CODE_FONT_SIZE,
    DEFAULT_UI_FONT_SIZE,
    UI_FONT_SIZES,
    codeFontStack,
    type AccentId,
    type CodeFontId,
  } from '$lib/state/appearance.svelte'
  import { appearance } from '$lib/state'

  function sizeLabel(size: number, fallback: number) {
    return size === fallback ? `${size}px (default)` : `${size}px`
  }

  function codeFontLabel(id: string) {
    const font = CODE_FONTS.find((row) => row.id === id)
    if (!font) return id
    return id === DEFAULT_CODE_FONT ? `${font.label} (default)` : font.label
  }
</script>

<SettingsPanel data-od-id="settings-appearance-group">
  <SettingsRow
    title="Theme"
    description="Follow the system, or always use light or dark"
  >
    {#snippet control()}
      <ToggleGroup.Root
        type="single"
        variant="outline"
        size="sm"
        aria-label="Theme"
        class="data-[spacing=0]:rounded-lg"
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
    title="Primary color"
    description="Used for primary buttons and highlights"
  >
    {#snippet control()}
      <RadioGroup.Root
        value={appearance.accent}
        onValueChange={(next) => appearance.setAccent(next as AccentId)}
        orientation="horizontal"
        aria-label="Primary color"
        class="flex items-center gap-2"
      >
        {#each ACCENTS as accent (accent.id)}
          <RadioGroup.Item
            value={accent.id}
            aria-label={accent.label}
            title={accent.label}
            class="flex size-7 items-center justify-center rounded-full ring-offset-2 ring-offset-card outline-none transition-shadow focus-visible:ring-2 focus-visible:ring-ring data-[state=checked]:ring-2 data-[state=checked]:ring-foreground/40"
            style="background: var(--swatch-{accent.id})"
          >
            {#snippet children({ checked })}
              {#if checked}
                <CheckIcon
                  class="size-3.5 {accent.id === 'neutral'
                    ? 'text-background'
                    : accent.id === 'gold'
                      ? 'text-black/80'
                      : 'text-white'}"
                  aria-hidden="true"
                />
              {/if}
            {/snippet}
          </RadioGroup.Item>
        {/each}
      </RadioGroup.Root>
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
        <Select.Trigger id="settings-ui-font-size" class="w-[150px]">
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
    title="Code font"
    description="Monospace for diffs, output, branches and code blocks"
    controlId="settings-code-font"
  >
    {#snippet control()}
      <Select.Root
        type="single"
        value={appearance.codeFont}
        onValueChange={(next) => appearance.setCodeFont(next as CodeFontId)}
      >
        <Select.Trigger
          id="settings-code-font"
          class="w-52"
          style="font-family: {codeFontStack(appearance.codeFont)}"
        >
          {codeFontLabel(appearance.codeFont)}
        </Select.Trigger>
        <Select.Content>
          {#each CODE_FONTS as font (font.id)}
            <Select.Item value={font.id} style="font-family: {font.stack}">
              {codeFontLabel(font.id)}
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
        <Select.Trigger id="settings-code-font-size" class="w-[150px]">
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
