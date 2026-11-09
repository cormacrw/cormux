/** `cat` or `cats`; pass the plural for words that don't just add an `s`. */
export function pluralWord(
  count: number,
  singular: string,
  plural = `${singular}s`,
): string {
  return count === 1 ? singular : plural
}

/** `1 cat`, `3 cats`. */
export function plural(
  count: number,
  singular: string,
  pluralForm = `${singular}s`,
): string {
  return `${count} ${pluralWord(count, singular, pluralForm)}`
}
