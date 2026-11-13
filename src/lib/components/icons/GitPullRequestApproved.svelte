<script lang="ts">
  import type { SVGAttributes } from 'svelte/elements'
  import { cn } from '$lib/utils'

  // Lucide has no approved-PR icon: this is its git-pull-request-create with the plus swapped
  // for a check, drawn with Lucide's own attributes so it sits beside the other PR icons.
  let { class: className, ...rest }: SVGAttributes<SVGSVGElement> = $props()
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  width="24"
  height="24"
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  class={cn('lucide-icon lucide lucide-git-pull-request-approved', className)}
  {...rest}
>
  <circle cx="6" cy="6" r="3" />
  <path d="M6 9v12" />
  <path d="M13 6h3a2 2 0 0 1 2 2v4" />
  <path d="m15 18 2 2 4-4" />
</svg>
