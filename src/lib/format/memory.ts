export function formatMemoryGb(bytes: number): string {
  const gb = bytes / 1024 ** 3
  if (gb >= 10) return `${Math.round(gb)} GB`
  return `${gb.toFixed(1)} GB`
}
