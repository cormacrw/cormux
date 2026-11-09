import { fileURLToPath } from 'node:url'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import { defineConfig, type Plugin } from 'vite'
import { bootAppearance } from './src/lib/state/appearance-tokens.ts'

const host = process.env.TAURI_DEV_HOST

// The diff viewer registers every highlight.js grammar, 190 of them; lowlight's `common` set
// covers the languages diffs here are in, for about a tenth of the size.
const commonDiffGrammars = (): Plugin => ({
  name: 'common-diff-grammars',
  enforce: 'pre',
  transform(code, id) {
    if (!id.includes('@git-diff-view')) return
    return code.replace(
      /import\s*\{\s*createLowlight,\s*all\s*\}\s*from\s*['"]lowlight['"]/,
      "import { createLowlight, common as all } from 'lowlight'",
    )
  },
})

// index.html applies the saved appearance before the app loads, from the app's own values.
const appearanceTokens = (): Plugin => ({
  name: 'appearance-tokens',
  transformIndexHtml(html) {
    return html.replace(
      '__APPEARANCE_TOKENS__',
      JSON.stringify(bootAppearance()),
    )
  },
})

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [appearanceTokens(), commonDiffGrammars(), tailwindcss(), svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
    },
  },
  // Vite's dependency scan misses this one: it's only reached through the lazy diff view. Found
  // on first use instead, it makes Vite re-bundle and reload every open page, which in e2e
  // reloads every test in flight.
  optimizeDeps: { include: ['@git-diff-view/svelte'] },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    target: 'safari16',
    minify: !process.env.TAURI_ENV_DEBUG,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
})
