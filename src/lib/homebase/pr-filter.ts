export type PrRelationship = 'review' | 'author' | 'mention' | 'assigned'

export type PrFilter = 'all' | 'review' | 'author'

export type PrFilterCounts = {
  all: number
  review: number
  author: number
}

export type PullRequestRow = {
  id: string
  rel: PrRelationship
}

export function matchesPrFilter(
  rel: PrRelationship,
  filter: PrFilter,
): boolean {
  if (filter === 'all') return true
  if (filter === 'review') return rel === 'review'
  return rel === 'author'
}

export function prFilterCounts(rows: PullRequestRow[]): PrFilterCounts {
  let review = 0
  let author = 0
  for (const row of rows) {
    if (row.rel === 'review') review += 1
    if (row.rel === 'author') author += 1
  }
  return { all: rows.length, review, author }
}

export function filterPullRequests<T extends PullRequestRow>(
  rows: T[],
  filter: PrFilter,
): T[] {
  return rows.filter((row) => matchesPrFilter(row.rel, filter))
}
