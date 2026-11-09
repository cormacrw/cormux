import js from '@eslint/js'
import svelte from 'eslint-plugin-svelte'
import globals from 'globals'
import ts from 'typescript-eslint'
import svelteConfig from './svelte.config.js'

export default ts.config(
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    rules: {
      'no-console': 'off',
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
    languageOptions: {
      parserOptions: {
        extraFileExtensions: ['.svelte'],
        parser: ts.parser,
        svelteConfig,
      },
    },
  },
  {
    files: [
      'src/lib/components/markdown/DocMarkdown.svelte',
      'src/lib/components/workspace/thread/reply/ReplyMarkdown.svelte',
      'src/lib/components/workspace/thread/reply/MathBlock.svelte',
      'src/lib/components/workspace/thread/reply/MermaidBlock.svelte',
    ],
    rules: {
      'svelte/no-at-html-tags': 'off',
    },
  },
  {
    files: ['src/**'],
    ignores: ['src/lib/feedback/open-external.ts'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: '@tauri-apps/plugin-opener',
              importNames: ['openUrl'],
              message:
                'Use openExternal from $lib/feedback/open-external, which toasts a failure.',
            },
          ],
        },
      ],
    },
  },
  {
    ignores: [
      'dist/**',
      'src-tauri/**',
      'src/lib/ipc/bindings.ts',
      'node_modules/**',
      'e2e/output/**',
      'playwright-report/**',
      'test-results/**',
    ],
  },
)
