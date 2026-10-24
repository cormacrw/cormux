import type { SceneModule } from './types'

/**
 * Background choices for Settings > Appearance. `load` pulls in the scene and
 * three.js on demand, so neither is on the boot path; `none` has no scene.
 * `preview` is a CSS background for the picker tile. It reads `--primary`
 * (the current accent) and `--tile-bg` (set by the tile per theme), so tiles
 * follow the accent without rendering any WebGL.
 */
export type BackgroundOption = {
  id: string
  label: string
  preview: string
  load: (() => Promise<SceneModule>) | null
}

const accent = (percent: number) =>
  `color-mix(in oklab, var(--primary) ${percent}%, var(--tile-bg))`

export const BACKGROUNDS = [
  {
    id: 'studio',
    label: 'Studio',
    preview: `radial-gradient(circle at 34% 46%, ${accent(95)} 0 13%, transparent 14%), radial-gradient(circle at 70% 40%, color-mix(in oklab, white 70%, var(--tile-bg)) 0 7%, transparent 8%), linear-gradient(180deg, var(--tile-bg) 0 62%, ${accent(14)} 62% 100%)`,
    load: () => import('./scenes/studio'),
  },
  {
    id: 'orbs',
    label: 'Orbs',
    preview: `radial-gradient(circle at 28% 36%, ${accent(90)} 0 14%, transparent 15%), radial-gradient(circle at 66% 62%, ${accent(55)} 0 19%, transparent 20%), radial-gradient(circle at 78% 24%, ${accent(30)} 0 8%, transparent 9%), var(--tile-bg)`,
    load: () => import('./scenes/orbs'),
  },
  {
    id: 'waves',
    label: 'Waves',
    preview: `radial-gradient(140% 60% at 30% 100%, ${accent(70)} 0 30%, transparent 31%), radial-gradient(140% 60% at 80% 115%, ${accent(40)} 0 40%, transparent 41%), linear-gradient(180deg, var(--tile-bg), ${accent(18)})`,
    load: () => import('./scenes/waves'),
  },
  {
    id: 'crystals',
    label: 'Crystals',
    preview: `conic-gradient(from 30deg at 40% 50%, ${accent(90)}, ${accent(40)}, ${accent(75)}, ${accent(25)}, ${accent(90)}) center / 46% 56% no-repeat, conic-gradient(from 200deg at 50% 50%, ${accent(60)}, ${accent(20)}, ${accent(60)}) 85% 25% / 20% 26% no-repeat, var(--tile-bg)`,
    load: () => import('./scenes/crystals'),
  },
  {
    id: 'particles',
    label: 'Particles',
    preview: `radial-gradient(circle, ${accent(90)} 0 1px, transparent 1.5px) 0 0 / 9px 9px, radial-gradient(ellipse at 50% 50%, ${accent(35)}, var(--tile-bg) 70%)`,
    load: () => import('./scenes/particles'),
  },
  {
    id: 'aurora',
    label: 'Aurora',
    preview: `linear-gradient(160deg, transparent 20%, ${accent(55)} 38%, transparent 52%), linear-gradient(200deg, transparent 40%, ${accent(35)} 58%, transparent 74%), var(--tile-bg)`,
    load: () => import('./scenes/aurora'),
  },
  {
    id: 'none',
    label: 'Gradient',
    preview: `radial-gradient(120% 90% at 20% 0%, ${accent(30)}, var(--tile-bg) 70%)`,
    load: null,
  },
] as const satisfies readonly BackgroundOption[]

export type BackgroundId = (typeof BACKGROUNDS)[number]['id']

export const DEFAULT_BACKGROUND: BackgroundId = 'studio'

export function isBackgroundId(value: unknown): value is BackgroundId {
  return BACKGROUNDS.some((row) => row.id === value)
}

export function backgroundOption(id: string): BackgroundOption {
  return BACKGROUNDS.find((row) => row.id === id) ?? BACKGROUNDS[0]
}
