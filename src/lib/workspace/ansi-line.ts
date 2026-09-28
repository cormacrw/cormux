/* eslint-disable no-control-regex -- ANSI stripping intentionally matches ESC sequences */
const ANSI_RE =
  /\u001b\[[0-9;]*m|\u001b\][^\u0007]*(?:\u0007|\u001b\\)|\u001b[PX^_][^\u001b]*?\u001b\\|\u001b.|\u009b[0-9;]*m/g

/** Strip ANSI control sequences for plain-text display. */
export function stripAnsi(line: string): string {
  return line.replace(ANSI_RE, '')
}
