import Anser from 'anser'

export function ansiLineToHtml(line: string): string {
  // ansiToHtml does not escape, so escape first or app output becomes markup.
  return Anser.ansiToHtml(Anser.escapeForHtml(line))
}
