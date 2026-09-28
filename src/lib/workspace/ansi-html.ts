import Anser from 'anser'

/** Convert PTY ANSI sequences to safe HTML spans for the log. */
export function ansiLineToHtml(line: string): string {
  return Anser.ansiToHtml(line)
}
