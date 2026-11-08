// Shims for `state/index.ts`, which another branch is changing. On merge, point it at
// `$lib/format/relative-time` and `$lib/state/pr-from-payload`, then delete `homebase/`.
export { parseTimestampMs } from '$lib/format/relative-time'
