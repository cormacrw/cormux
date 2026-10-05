/** Dunder Mifflin. A thread keeps whichever one it hashes to. */
const OFFICE_NAMES = [
  'Michael',
  'Dwight',
  'Jim',
  'Pam',
  'Angela',
  'Kevin',
  'Oscar',
  'Stanley',
  'Phyllis',
  'Meredith',
  'Creed',
  'Kelly',
  'Ryan',
  'Toby',
  'Andy',
  'Erin',
  'Darryl',
  'Holly',
] as const

export function agentName(threadId: string): string {
  let hash = 0
  for (let i = 0; i < threadId.length; i++) {
    hash = (Math.imul(hash, 31) + threadId.charCodeAt(i)) >>> 0
  }
  // ponytail: two threads can share a name. Deal from a taken set if that shows up.
  return OFFICE_NAMES[hash % OFFICE_NAMES.length] ?? OFFICE_NAMES[0]
}
