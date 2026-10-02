// From Svelte Bits (TypeScript + Tailwind). `onEnd` waits for the spring to
// settle rather than a `delay + duration` timer, which fired well before it did.
import Root from './count-up.svelte'

export {
  Root,
  //
  Root as CountUp,
}
