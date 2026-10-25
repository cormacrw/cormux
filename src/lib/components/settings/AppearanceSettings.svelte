<script lang="ts">
  import { RadioGroup } from 'bits-ui'
  import CheckIcon from '@lucide/svelte/icons/check'
  import { setMode, userPrefersMode } from 'mode-watcher'
  import * as Select from '$lib/components/ui/select'
  import { Switch } from '$lib/components/ui/switch'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte'
  import SettingsRow from '$lib/components/settings/SettingsRow.svelte'
  import {
    CODE_FONTS,
    CODE_FONT_SIZES,
    DEFAULT_CODE_FONT,
    DEFAULT_CODE_FONT_SIZE,
    DEFAULT_UI_FONT_SIZE,
    UI_FONT_SIZES,
    THEMES,
    codeFontStack,
    type CodeFontId,
    type ThemeId,
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
    title="Mode"
    description="Follow the system, or always use light or dark"
  >
    {#snippet control()}
      <ToggleGroup.Root
        type="single"
        variant="outline"
        size="sm"
        aria-label="Mode"
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
    title="Theme"
    description="Colours the window, the glass and the buttons"
    class="sm:flex-col sm:items-stretch"
  >
    {#snippet control()}
      <RadioGroup.Root
        value={appearance.theme}
        onValueChange={(next) => appearance.setTheme(next as ThemeId)}
        aria-label="Theme"
        class="grid grid-cols-[repeat(auto-fill,minmax(6.5rem,1fr))] gap-2"
      >
        {#each THEMES as theme (theme.id)}
          <RadioGroup.Item
            value={theme.id}
            class="group flex flex-col gap-1.5 rounded-lg p-1 text-left outline-none ring-offset-2 ring-offset-card transition-shadow focus-visible:ring-2 focus-visible:ring-ring"
          >
            {#snippet children({ checked })}
              <span
                class="theme-tile relative block aspect-[16/10] w-full overflow-hidden rounded-md border border-border transition-shadow {checked
                  ? 'ring-2 ring-primary'
                  : 'group-hover:ring-1 group-hover:ring-foreground/20'}"
                data-neutral={theme.hue == null}
                style:--tile-h={theme.hue ?? 265}
                style:--tile-s={'strength' in theme ? theme.strength : 1}
                aria-hidden="true"
              >
                <span class="tile-glass"></span>
                <span class="tile-dot"></span>
              </span>
              <span
                class="flex items-center gap-1 px-0.5 text-xs {checked
                  ? 'font-medium text-foreground'
                  : 'text-muted-foreground'}"
              >
                {theme.label}
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
    title="Wordmark"
    description="Write “cormux” large and faint behind the glass"
  >
    {#snippet control()}
      <Switch
        checked={appearance.showWordmark}
        onCheckedChange={(next) => appearance.setShowWordmark(next)}
        aria-label="Show wordmark"
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
          class="w-64 max-w-full"
          style="font-family: {codeFontStack(appearance.codeFont)}"
        >
          <!-- Monospace runs wide; the box fits the longest name and truncates past it. -->
          <span class="min-w-0 truncate"
            >{codeFontLabel(appearance.codeFont)}</span
          >
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

<style>
  /* A tiny window in the theme: its gradient, a pane of glass and a primary dot. */
  .theme-tile {
    background:
      radial-gradient(
        120% 110% at 90% -10%,
        oklch(0.86 calc(0.09 * var(--tile-s)) var(--tile-h)),
        transparent 70%
      ),
      oklch(0.72 calc(0.13 * var(--tile-s)) var(--tile-h));
  }

  :global(.dark) .theme-tile {
    background:
      radial-gradient(
        120% 110% at 90% -10%,
        oklch(0.42 calc(0.17 * var(--tile-s)) var(--tile-h)),
        transparent 70%
      ),
      oklch(0.25 calc(0.12 * var(--tile-s)) var(--tile-h));
  }

  .theme-tile[data-neutral='true'] {
    background: linear-gradient(
      160deg,
      oklch(0.97 0.006 265),
      oklch(0.9 0.01 265)
    );
  }

  :global(.dark) .theme-tile[data-neutral='true'] {
    background: linear-gradient(
      160deg,
      oklch(0.22 0.015 265),
      oklch(0.13 0.012 265)
    );
  }

  .tile-glass {
    position: absolute;
    inset: 12% auto 12% 8%;
    width: 30%;
    border-radius: 4px;
    background: oklch(1 0 0 / 45%);
    box-shadow: inset 0 0 0 0.5px oklch(1 0 0 / 60%);
  }

  :global(.dark) .tile-glass {
    background: oklch(1 0 0 / 10%);
    box-shadow: inset 0 0 0 0.5px oklch(1 0 0 / 14%);
  }

  .tile-dot {
    position: absolute;
    right: 12%;
    bottom: 16%;
    width: 18%;
    aspect-ratio: 1;
    border-radius: 999px;
    background: oklch(0.52 0.2 var(--tile-h));
  }

  :global(.dark) .tile-dot {
    background: oklch(0.64 0.17 var(--tile-h));
  }

  .theme-tile[data-neutral='true'] .tile-dot {
    background: oklch(0.56 0.19 257);
  }
</style>
