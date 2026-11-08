/** What an engine reported for a thread; null where it reported nothing. */
export type ThreadUsage = {
  /** Tokens the latest turn's context held. */
  usedTokens: number | null
  /** The model's context window; 0 or null when the engine doesn't say. */
  contextSize: number | null
  /** What the thread has cost so far, in US dollars. */
  costUsd: number | null
}

export const NO_USAGE: ThreadUsage = {
  usedTokens: null,
  contextSize: null,
  costUsd: null,
}

/** `950`, `22.3k`, `1.2M`. */
function formatTokens(count: number): string {
  if (count < 1000) return String(count)
  if (count < 1_000_000) return `${trimZero((count / 1000).toFixed(1))}k`
  return `${trimZero((count / 1_000_000).toFixed(1))}M`
}

function trimZero(value: string): string {
  return value.endsWith('.0') ? value.slice(0, -2) : value
}

/** `$0.42`, `$12.30`, or `<$0.01` for a cost that rounds to nothing. */
export function formatCost(usd: number): string {
  if (usd > 0 && usd < 0.01) return '<$0.01'
  return `$${usd.toFixed(2)}`
}

/** How full the context is, as a whole percent, or null when the window isn't known. */
export function contextPercent(usage: ThreadUsage): number | null {
  if (usage.usedTokens == null || !usage.contextSize) return null
  return Math.min(100, Math.round((usage.usedTokens / usage.contextSize) * 100))
}

/**
 * The usage meter's tooltip (`22.3k tokens in context (2% of 1M) · $0.42 so far`), which says
 * what the engine didn't report rather than showing zero.
 */
export function usageDetail(usage: ThreadUsage): string {
  const percent = contextPercent(usage)
  const tokens = usage.usedTokens
  if (tokens == null && usage.costUsd == null) return 'Usage not reported'
  const context =
    tokens == null
      ? 'Tokens not reported'
      : percent == null
        ? `${formatTokens(tokens)} tokens in context`
        : `${formatTokens(tokens)} tokens in context (${percent}% of ${formatTokens(usage.contextSize!)})`
  const cost =
    usage.costUsd == null
      ? 'cost not reported'
      : `${formatCost(usage.costUsd)} so far`
  return `${context} · ${cost}`
}

/** What the threads have cost together, or null if none reported a cost. */
export function totalCost(usages: ThreadUsage[]): number | null {
  const costs = usages
    .map((usage) => usage.costUsd)
    .filter((cost): cost is number => cost != null)
  return costs.length ? costs.reduce((sum, cost) => sum + cost, 0) : null
}
