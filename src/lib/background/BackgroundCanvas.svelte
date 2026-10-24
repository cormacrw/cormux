<!--
  Full-window live background behind the glass UI. Shows a static accent
  gradient straight away, then lazily loads three.js and the chosen scene and
  fades its canvas in on top. Falls back to the gradient for `none`, when
  WebGL is unavailable, or if the context is lost.

  Rendering is throttled to ~30fps, paused while the window is hidden or
  unfocused, and reduced to a single still frame under reduced motion or
  when "Animate background" is off.
-->
<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import type { WebGLRenderer } from 'three'
  import { appearance } from '$lib/state/appearance.svelte'
  import { backgroundOption } from './registry'
  import type { CreateScene, Scene } from './types'

  let {
    background,
    accent,
    animate,
  }: {
    /** Scene id; defaults to `appearance.background`. */
    background?: string
    /** Accent hex; defaults to `appearance.accentHex`. */
    accent?: string
    /** Whether to animate; defaults to `appearance.animateBackground`. */
    animate?: boolean
  } = $props()

  const sceneId = $derived(background ?? appearance.background)
  const accentHex = $derived(accent ?? appearance.accentHex)
  const animated = $derived(animate ?? appearance.animateBackground)

  const MAX_PIXEL_RATIO = 1.5
  const FRAME_MS = 1000 / 30
  /** Matches the canvas opacity transition below. */
  const FADE_MS = 600

  type Active = {
    canvas: HTMLCanvasElement
    renderer: WebGLRenderer
    scene: Scene
  }

  let host: HTMLDivElement
  let renderMode = $state<'static' | 'webgl'>('static')

  let active: Active | null = null
  let token = 0
  let mounted = false
  let firstStart = true
  /** Canvases fading out, freed when their timer fires or on unmount. */
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- bookkeeping, not UI state
  const retiring = new Map<ReturnType<typeof setTimeout>, Active>()

  let dark = false
  let reducedMotion = false
  let visible = true
  let focused = true
  let raf = 0
  let lastTick = 0
  let lastFrame = 0
  let time = 0
  let dirty = true
  let motionQuery: MediaQueryList | null = null

  function readRoot() {
    const root = document.documentElement
    dark = root.classList.contains('dark')
    reducedMotion =
      root.classList.contains('reduce-motion') || !!motionQuery?.matches
  }

  function running() {
    return animated && !reducedMotion && visible && focused
  }

  function frame(now: number) {
    raf = 0
    const current = active
    if (!current) return
    if (running()) {
      raf = requestAnimationFrame(frame)
      if (lastFrame && now - lastFrame < FRAME_MS - 2) return
      // Cap the step so a long stall doesn't jump the motion.
      if (lastTick) time += Math.min(now - lastTick, 100) / 1000
      lastTick = now
      lastFrame = now
      current.scene.render(time)
      dirty = false
      return
    }
    lastTick = 0
    lastFrame = 0
    if (dirty) current.scene.render(time)
    dirty = false
  }

  function schedule() {
    if (!raf && active) raf = requestAnimationFrame(frame)
  }

  function invalidate() {
    dirty = true
    schedule()
  }

  function destroy(entry: Active) {
    try {
      entry.scene.dispose()
    } catch {
      // A scene that fails to clean up shouldn't keep the renderer alive.
    }
    entry.renderer.dispose()
    entry.renderer.forceContextLoss()
    entry.canvas.remove()
  }

  /** Fade a canvas out, then free it. */
  function retire(entry: Active | null) {
    if (!entry) return
    entry.canvas.classList.remove('is-visible')
    const timer = setTimeout(() => {
      retiring.delete(timer)
      destroy(entry)
    }, FADE_MS)
    retiring.set(timer, entry)
  }

  function fallBack() {
    retire(active)
    active = null
    renderMode = 'static'
  }

  /** Let first paint and boot work finish before pulling in three.js. */
  function idle() {
    return new Promise<void>((resolve) => {
      const ric = (
        window as Window & {
          requestIdleCallback?: (
            cb: () => void,
            opts?: { timeout: number },
          ) => number
        }
      ).requestIdleCallback
      if (ric) ric(() => resolve(), { timeout: 800 })
      else setTimeout(resolve, 200)
    })
  }

  async function start(id: string) {
    const mine = ++token
    const option = backgroundOption(id)
    if (!option.load) {
      fallBack()
      return
    }
    if (firstStart) {
      firstStart = false
      await idle()
      if (mine !== token || !mounted) return
    }

    let create: CreateScene
    let THREE: typeof import('three')
    try {
      const [module, three] = await Promise.all([
        option.load(),
        import('three'),
      ])
      create = module.create
      THREE = three
    } catch {
      if (mine === token) fallBack()
      return
    }
    if (mine !== token || !mounted) return

    const width = window.innerWidth
    const height = window.innerHeight
    const canvas = document.createElement('canvas')
    let renderer: WebGLRenderer
    try {
      renderer = new THREE.WebGLRenderer({
        canvas,
        antialias: true,
        alpha: false,
        stencil: false,
        powerPreference: 'low-power',
      })
    } catch {
      // No WebGL (e.g. headless browsers): keep the static gradient.
      fallBack()
      return
    }
    renderer.setPixelRatio(
      Math.min(window.devicePixelRatio || 1, MAX_PIXEL_RATIO),
    )
    renderer.setSize(width, height, false)
    renderer.toneMapping = THREE.NeutralToneMapping

    let scene: Scene
    try {
      scene = create({
        renderer,
        width,
        height,
        accent: untrack(() => accentHex),
        dark,
      })
    } catch {
      renderer.dispose()
      renderer.forceContextLoss()
      fallBack()
      return
    }

    const entry: Active = { canvas, renderer, scene }
    canvas.addEventListener('webglcontextlost', (event) => {
      event.preventDefault()
      if (active === entry) fallBack()
    })
    // The host has no Svelte-managed children; canvases are ours alone.
    // eslint-disable-next-line svelte/no-dom-manipulating
    host.append(canvas)

    const previous = active
    active = entry
    renderMode = 'webgl'
    try {
      scene.render(time)
    } catch {
      fallBack()
      return
    }
    // Next frame, so the opacity transition runs from 0.
    requestAnimationFrame(() => canvas.classList.add('is-visible'))
    retire(previous)
    dirty = false
    schedule()
  }

  function onResize() {
    if (!active) return
    const width = window.innerWidth
    const height = window.innerHeight
    active.renderer.setPixelRatio(
      Math.min(window.devicePixelRatio || 1, MAX_PIXEL_RATIO),
    )
    active.renderer.setSize(width, height, false)
    active.scene.resize(width, height)
    invalidate()
  }

  function onVisibility() {
    visible = !document.hidden
    invalidate()
  }

  function onFocus() {
    focused = true
    invalidate()
  }

  function onBlur() {
    focused = false
  }

  function onRootChange() {
    const wasDark = dark
    readRoot()
    if (active && wasDark !== dark) active.scene.setTheme(dark)
    invalidate()
  }

  onMount(() => {
    mounted = true
    motionQuery = window.matchMedia('(prefers-reduced-motion: reduce)')
    readRoot()
    visible = !document.hidden
    focused = document.hasFocus()

    const observer = new MutationObserver(onRootChange)
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    })
    motionQuery.addEventListener('change', onRootChange)
    window.addEventListener('resize', onResize)
    window.addEventListener('focus', onFocus)
    window.addEventListener('blur', onBlur)
    document.addEventListener('visibilitychange', onVisibility)

    return () => {
      mounted = false
      token++
      observer.disconnect()
      motionQuery?.removeEventListener('change', onRootChange)
      window.removeEventListener('resize', onResize)
      window.removeEventListener('focus', onFocus)
      window.removeEventListener('blur', onBlur)
      document.removeEventListener('visibilitychange', onVisibility)
      if (raf) cancelAnimationFrame(raf)
      raf = 0
      for (const [timer, entry] of retiring) {
        clearTimeout(timer)
        destroy(entry)
      }
      retiring.clear()
      if (active) destroy(active)
      active = null
    }
  })

  // Switch scenes when the choice changes (and start the first one).
  $effect(() => {
    const id = sceneId
    untrack(() => void start(id))
  })

  // Recolour the running scene in place.
  $effect(() => {
    const hex = accentHex
    untrack(() => {
      if (!active) return
      active.scene.setAccent(hex)
      invalidate()
    })
  })

  // Resume or freeze when "Animate background" flips.
  $effect(() => {
    void animated
    untrack(invalidate)
  })
</script>

<div
  bind:this={host}
  class="cormux-background"
  aria-hidden="true"
  data-background={sceneId}
  data-render-mode={renderMode}
  style:--bg-accent={accentHex}
></div>

<style>
  .cormux-background {
    --bg-base: #eef0f3;
    position: fixed;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    overflow: hidden;
    background-color: var(--bg-base);
    background-image:
      radial-gradient(
        120% 90% at 15% 0%,
        color-mix(in oklab, var(--bg-accent) 24%, transparent),
        transparent 65%
      ),
      radial-gradient(
        90% 80% at 100% 100%,
        color-mix(in oklab, var(--bg-accent) 14%, transparent),
        transparent 70%
      );
  }

  :global(html.dark) .cormux-background {
    --bg-base: #0b0c0e;
  }

  .cormux-background :global(canvas) {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
    opacity: 0;
    transition: opacity 600ms ease;
  }

  .cormux-background :global(canvas.is-visible) {
    opacity: 1;
  }
</style>
