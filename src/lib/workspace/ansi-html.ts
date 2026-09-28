import Anser from 'anser'

/** Convert PTY ANSI sequences to safe HTML spans for the log. */
export function ansiLineToHtml(line: string): string {
  // ansiToHtml does not escape, so escape first or app output becomes markup.
  return Anser.ansiToHtml(Anser.escapeForHtml(line))
}
