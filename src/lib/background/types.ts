import type { WebGLRenderer } from 'three'

/** What BackgroundCanvas hands a scene when it starts one. */
export type SceneContext = {
  /** Already sized and configured; the scene must not dispose it. */
  renderer: WebGLRenderer
  width: number
  height: number
  /** Resolved accent hex, e.g. `#1170e0`. */
  accent: string
  /** True when `<html>` has the `dark` class. */
  dark: boolean
}

/**
 * A running background. Every method is cheap and allocation free on the hot
 * path; `setAccent` and `setTheme` update materials in place and never
 * rebuild the scene.
 */
export type Scene = {
  setAccent(hex: string): void
  setTheme(dark: boolean): void
  resize(width: number, height: number): void
  /** Draw one frame at `time` seconds of animation. */
  render(time: number): void
  /** Free geometries, materials and textures (not the renderer). */
  dispose(): void
}

export type CreateScene = (ctx: SceneContext) => Scene

export type SceneModule = { create: CreateScene }
