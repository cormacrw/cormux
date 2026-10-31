import { fileURLToPath } from 'node:url'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import { defineConfig, type Plugin } from 'vite'

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

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [commonDiffGrammars(), tailwindcss(), svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
    },
  },
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
