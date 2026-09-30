/** `pnpm tauri dev` serves through Vite's dev server; `pnpm tauri build` doesn't. */
export const isDevBuild = import.meta.env.DEV

/** Dev builds say so everywhere the app names itself, so they never pass for the installed app. */
export const appName = isDevBuild ? 'Cormux Dev' : 'Cormux'
