/** KaTeX with its fonts and styles, loaded the first time a reply has math. */
export async function renderMath(
  tex: string,
  display: boolean,
): Promise<string> {
  const [{ default: katex }] = await Promise.all([
    import('katex'),
    import('katex/dist/katex.min.css'),
  ])
  // `trust` stays off, so agent TeX can't produce links or raw HTML.
  return katex.renderToString(tex, {
    displayMode: display,
    throwOnError: false,
    trust: false,
  })
}
