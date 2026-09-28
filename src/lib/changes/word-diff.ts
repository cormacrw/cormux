/** Word-level highlight ranges when a removed line is replaced by an added line. */
export function wordChangeRanges(
  removed: string,
  added: string,
): [[number, number] | null, [number, number] | null] {
  const min = Math.min(removed.length, added.length)
  let prefix = 0
  while (prefix < min && removed[prefix] === added[prefix]) prefix++
  let suffix = 0
  while (
    suffix < min - prefix &&
    removed[removed.length - 1 - suffix] === added[added.length - 1 - suffix]
  ) {
    suffix++
  }
  const ra: [number, number] = [prefix, removed.length - suffix]
  const rb: [number, number] = [prefix, added.length - suffix]
  const tooBig = (range: [number, number], text: string) => {
    const span = range[1] - range[0]
    return span <= 0 || span / Math.max(text.length, 1) > 0.8
  }
  return [
    tooBig(ra, removed) ? null : ra,
    tooBig(rb, added) ? null : rb,
  ]
}
