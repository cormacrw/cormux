// What a test or build run came to, read from the summary lines the common runners print.

export type RunSummary = {
  passed: number
  failed: number
  skipped: number
  /** Compiler or type errors, for a build rather than a test run. */
  errors: number
  /** The names of what failed, as far as the output says. */
  failures: string[]
}

const MAX_FAILURES = 8

const ANSI = new RegExp(String.raw`\u001b\[[0-9;]*m`, 'g')

const FAILURE_LINES = [
  /^\s*(?:FAIL|FAILED)\s+(\S.*)$/,
  /^\s*[×✗✕]\s+(\S.*)$/,
  /^\s*\d+\)\s+(\S.*)$/,
  /^test (\S+) \.\.\. FAILED$/,
]

function count(line: string, word: RegExp): number {
  return Number(word.exec(line)?.[1] ?? 0)
}

/** Null when the output isn't a run any of them would recognise. */
export function summarizeRun(output: string): RunSummary | null {
  const summary: RunSummary = {
    passed: 0,
    failed: 0,
    skipped: 0,
    errors: 0,
    failures: [],
  }
  let found = false
  const addFrom = (line: string) => {
    summary.passed += count(line, /(\d+) passed/)
    summary.failed += count(line, /(\d+) failed/)
    summary.skipped += count(line, /(\d+) (?:skipped|ignored)/)
    found = true
  }

  for (const line of output.replace(ANSI, '').split('\n')) {
    if (/^\s*Tests:?\s+.*\d+ (?:passed|failed)/.test(line)) addFrom(line)
    // cargo prints one of these per test binary.
    else if (/^test result: /.test(line)) addFrom(line)
    // pytest: `=== 2 failed, 10 passed in 1.20s ===`.
    else if (/^=+ .*\d+ (?:passed|failed).* =+$/.test(line)) addFrom(line)
    // Playwright prints its counts on lines of their own.
    else if (/^\s*\d+ (?:passed|failed|skipped)(?: \(.+\))?\s*$/.test(line)) {
      addFrom(line)
    }
    if (/^error(?:\[E\d+\])?: /.test(line) || /: error TS\d+:/.test(line)) {
      summary.errors += 1
      found = true
    }
    for (const pattern of FAILURE_LINES) {
      const name = pattern.exec(line)?.[1]?.trim()
      if (!name || summary.failures.includes(name)) continue
      if (summary.failures.length < MAX_FAILURES) summary.failures.push(name)
      break
    }
  }
  return found ? summary : null
}

/** `214 passed, 2 failed`. */
export function runSummaryLabel(summary: RunSummary): string {
  const plural = (n: number, word: string) =>
    `${n} ${word}${n === 1 ? '' : 's'}`
  return (
    [
      summary.passed ? `${summary.passed} passed` : '',
      summary.failed ? `${summary.failed} failed` : '',
      summary.skipped ? `${summary.skipped} skipped` : '',
      summary.errors ? plural(summary.errors, 'error') : '',
    ]
      .filter(Boolean)
      .join(', ') || 'No tests ran'
  )
}

export function runSucceeded(summary: RunSummary): boolean {
  return summary.failed === 0 && summary.errors === 0
}
