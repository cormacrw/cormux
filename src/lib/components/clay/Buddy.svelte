<script lang="ts">
  import type { BuddyFace } from '$lib/clay/identity'
  import { cn } from '$lib/utils'

  /** A clay buddy. Its face is the status; `none` is the plain mound used as a row bead. */
  let {
    color,
    face = 'happy',
    size = 40,
    breathe = false,
    class: className = '',
  }: {
    color: string
    face?: BuddyFace
    size?: number
    breathe?: boolean
    class?: string
  } = $props()

  const ink = '#3B2A22'
</script>

<svg
  width={size}
  height={Math.round((size * 60) / 64)}
  viewBox="0 0 68 64"
  aria-hidden="true"
  class={cn('buddy shrink-0', breathe && 'clay-breathe', className)}
>
  <path
    d="M8 40c0-18 11-32 26-32s26 14 26 32c0 13-10 18-26 18S8 53 8 40z"
    fill={color}
  />
  <path
    d="M8 40c0 13 10 18 26 18s26-5 26-18c-4 8-12 11-26 11S12 48 8 40z"
    fill="#000"
    opacity=".14"
  />
  <ellipse
    cx="24"
    cy="22"
    rx="8"
    ry="5"
    fill="#FFF"
    opacity=".55"
    transform="rotate(-25 24 22)"
  />
  {#if face === 'happy'}
    <path
      d="M27 32v2M41 32v2"
      stroke={ink}
      stroke-width="4"
      stroke-linecap="round"
      fill="none"
    />
    <path
      d="M28 42q6 5 12 0"
      stroke={ink}
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
  {:else if face === 'gasp'}
    <path
      d="M27 29v6M41 29v6"
      stroke={ink}
      stroke-width="4"
      stroke-linecap="round"
      fill="none"
    />
    <path
      d="M32 44a2.5 3 0 1 0 5 0a2.5 3 0 1 0-5 0"
      stroke={ink}
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
  {:else if face === 'flat'}
    <path
      d="M23 33h8M37 33h8"
      stroke={ink}
      stroke-width="4"
      stroke-linecap="round"
      fill="none"
    />
    <path
      d="M30 44q4-2 8 0"
      stroke={ink}
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
  {:else if face === 'calm'}
    <path
      d="M27 32v2M41 32v2"
      stroke={ink}
      stroke-width="4"
      stroke-linecap="round"
      fill="none"
    />
    <path
      d="M30 44h8"
      stroke={ink}
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
  {:else if face === 'sleepy'}
    <path
      d="M23 33q4 3 8 0M37 33q4 3 8 0"
      stroke={ink}
      stroke-width="4"
      stroke-linecap="round"
      fill="none"
    />
    <path
      d="M31 44h6"
      stroke={ink}
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
  {:else if face === 'glasses'}
    <circle
      cx="26"
      cy="33"
      r="7"
      fill="#FFF6E6"
      stroke={ink}
      stroke-width="2.5"
    />
    <circle
      cx="42"
      cy="33"
      r="7"
      fill="#FFF6E6"
      stroke={ink}
      stroke-width="2.5"
    />
    <path d="M33 33h2" stroke={ink} stroke-width="2.5" />
    <circle cx="26" cy="33" r="2.5" fill={ink} />
    <circle cx="42" cy="33" r="2.5" fill={ink} />
    <path
      d="M28 44q6 5 12 0"
      stroke={ink}
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
  {/if}
</svg>

<style>
  .buddy {
    filter: drop-shadow(0 3px 2.5px rgb(30 40 40 / 0.28));
  }
</style>
