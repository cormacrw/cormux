<!--
  Behind the glass: the theme's gradient, and "cormux" set huge and faint on a
  diagonal across the content area in Pacifico (OFL). Plain CSS, so the
  gradient paints with the first frame.
-->
<script lang="ts">
  import { appearance } from '$lib/state/appearance.svelte'

  /** Rising from bottom left to top right. */
  const TILT_DEG = -20
  const COS = Math.cos((TILT_DEG * Math.PI) / 180)
  const SIN = Math.abs(Math.sin((TILT_DEG * Math.PI) / 180))

  let area: HTMLDivElement | undefined = $state()
  let mark: HTMLSpanElement | undefined = $state()
  /** Font size that fits the tilted word in the area; 0 until measured. */
  let size = $state(0)

  // Measure the word once its font is in, and fit it to the area either way.
  function fit() {
    if (!area || !mark) return
    const current = parseFloat(getComputedStyle(mark).fontSize) || 100
    const w = mark.offsetWidth / current
    const h = mark.offsetHeight / current
    if (!w || !h) return
    size = Math.min(
      (0.92 * area.clientWidth) / (w * COS + h * SIN),
      (0.9 * area.clientHeight) / (w * SIN + h * COS),
    )
  }

  $effect(() => {
    if (!appearance.showWordmark) return
    let cancelled = false
    void import('@fontsource/pacifico/400.css')
      .then(() => document.fonts.load("100px 'Pacifico'", 'cormux'))
      .catch(() => undefined)
      .then(() => {
        if (!cancelled) fit()
      })
    return () => {
      cancelled = true
    }
  })

  $effect(() => {
    if (!area) return
    const observer = new ResizeObserver(fit)
    observer.observe(area)
    return () => observer.disconnect()
  })
</script>

<div class="cormux-backdrop" aria-hidden="true" data-od-id="backdrop">
  {#if appearance.showWordmark}
    <div class="wordmark-area" bind:this={area}>
      <span
        bind:this={mark}
        class="wordmark"
        data-od-id="wordmark"
        style:font-size="{size || 100}px"
        style:opacity={size ? 1 : 0}
        style:--tilt="{TILT_DEG}deg">cormux</span
      >
    </div>
  {/if}
</div>

<style>
  .cormux-backdrop {
    position: fixed;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    overflow: hidden;
    background:
      radial-gradient(
        120% 110% at 90% -10%,
        var(--backdrop-hi),
        transparent 70%
      ),
      var(--backdrop);
    transition: background-color var(--dur-slide) ease;
  }

  /* The content area, right of the floating sidebar (its 256px column plus a little room). */
  .wordmark-area {
    position: absolute;
    inset: 0 0 0 264px;
  }

  .wordmark {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%) rotate(var(--tilt));
    font-family: 'Pacifico', cursive;
    line-height: 1;
    white-space: nowrap;
    color: var(--wordmark);
    user-select: none;
    transition: opacity var(--dur-med) ease;
  }
</style>
