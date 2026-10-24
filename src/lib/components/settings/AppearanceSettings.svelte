<script lang="ts">
  import { RadioGroup } from 'bits-ui'
  import CheckIcon from '@lucide/svelte/icons/check'
  import { setMode, userPrefersMode } from 'mode-watcher'
  import * as Select from '$lib/components/ui/select'
  import { Switch } from '$lib/components/ui/switch'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import { BACKGROUNDS, type BackgroundId } from '$lib/background/registry'
  import { readableForeground } from '$lib/background/color'
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

  const customActive = $derived(appearance.accent === 'custom')
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
    description="Used for primary buttons, highlights and the background scene"
  >
    {#snippet control()}
      <div class="flex items-center gap-2">
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
        <label
          title="Custom color"
          class="relative flex size-7 cursor-pointer items-center justify-center rounded-full ring-offset-2 ring-offset-card transition-shadow has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring {customActive
            ? 'ring-2 ring-foreground/40'
            : ''}"
          style:background={customActive
            ? appearance.customAccent
            : 'conic-gradient(from 90deg, #e5484d, #f5d90a, #30a46c, #0091ff, #8e4ec6, #e5484d)'}
        >
          <input
            type="color"
            aria-label="Custom color"
            data-testid="custom-accent"
            class="absolute inset-0 size-full cursor-pointer rounded-full opacity-0"
            value={appearance.customAccent}
            oninput={(event) =>
              appearance.setCustomAccent(event.currentTarget.value)}
          />
          {#if customActive}
            <CheckIcon
              class="pointer-events-none size-3.5"
              style="color: {readableForeground(appearance.customAccent)}"
              aria-hidden="true"
            />
          {/if}
        </label>
      </div>
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="Background"
    description="Shown behind the app's glass panels"
    class="sm:flex-col sm:items-stretch"
  >
    {#snippet control()}
      <RadioGroup.Root
        value={appearance.background}
        onValueChange={(next) => appearance.setBackground(next as BackgroundId)}
        aria-label="Background"
        class="grid grid-cols-[repeat(auto-fill,minmax(6.5rem,1fr))] gap-2 [--tile-bg:#eef0f3] dark:[--tile-bg:#0b0c0e]"
      >
        {#each BACKGROUNDS as option (option.id)}
          <RadioGroup.Item
            value={option.id}
            class="group flex flex-col gap-1.5 rounded-lg p-1 text-left outline-none ring-offset-2 ring-offset-card transition-shadow focus-visible:ring-2 focus-visible:ring-ring"
          >
            {#snippet children({ checked })}
              <span
                class="block aspect-[16/10] w-full rounded-md border border-border transition-shadow {checked
                  ? 'ring-2 ring-primary'
                  : 'group-hover:ring-1 group-hover:ring-foreground/20'}"
                style:background={option.preview}
                aria-hidden="true"
              ></span>
              <span
                class="flex items-center gap-1 px-0.5 text-xs {checked
                  ? 'font-medium text-foreground'
                  : 'text-muted-foreground'}"
              >
                {option.label}
                {#if checked}
                  <CheckIcon class="size-3" aria-hidden="true" />
                {/if}
              </span>
            {/snippet}
          </RadioGroup.Item>
        {/each}
      </RadioGroup.Root>
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="Animate background"
    description="Slow motion in the background scene. Pauses when Cormux is in the background, and stays still when reduced motion is on"
  >
    {#snippet control()}
      <Switch
        checked={appearance.animateBackground}
        onCheckedChange={(next) => appearance.setAnimateBackground(next)}
        aria-label="Animate background"
      />
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
