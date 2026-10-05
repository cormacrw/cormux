<script lang="ts">
  import type { BuddyMood } from '$lib/buddy'

  let {
    mood = 'idle',
    color,
    glasses = false,
    breathe = true,
    size = 20,
  }: {
    mood?: BuddyMood
    color: string
    glasses?: boolean
    breathe?: boolean
    size?: number
  } = $props()

  const height = $derived(Math.round((size * 64) / 68))
</script>

<svg
  width={size}
  {height}
  viewBox="0 0 68 64"
  aria-hidden="true"
  class={mood === 'running' && breathe ? 'buddy-breathe' : ''}
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
  {#if mood === 'needsYou'}
    <path
      d="M27 29v6M41 29v6"
      stroke="#3B2A22"
      stroke-width="4"
      stroke-linecap="round"
      fill="none"
    />
    <path
      d="M32 44a2.5 3 0 1 0 5 0a2.5 3 0 1 0-5 0"
      stroke="#3B2A22"
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
  {:else if mood === 'paused'}
    <path
      d="M23 33h8M37 33h8"
      stroke="#3B2A22"
      stroke-width="4"
      stroke-linecap="round"
      fill="none"
    />
    <path
      d="M30 44q4-2 8 0"
      stroke="#3B2A22"
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
  {:else if mood === 'starting'}
    <path
      d="M27 32v2M41 32v2"
      stroke="#3B2A22"
      stroke-width="4"
      stroke-linecap="round"
      fill="none"
    />
    <path
      d="M30 44h8"
      stroke="#3B2A22"
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
  {:else if mood === 'idle'}
    <path
      d="M23 33q4 3 8 0M37 33q4 3 8 0"
      stroke="#3B2A22"
      stroke-width="4"
      stroke-linecap="round"
      fill="none"
    />
    <path
      d="M31 44h6"
      stroke="#3B2A22"
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
  {:else}
    <path
      d="M27 32v2M41 32v2"
      stroke="#3B2A22"
      stroke-width="4"
      stroke-linecap="round"
      fill="none"
    />
    <path
      d="M28 42q6 5 12 0"
      stroke="#3B2A22"
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
  {/if}
  {#if glasses}
    <circle
      cx="26"
      cy="33"
      r="7"
      fill="none"
      stroke="#3B2A22"
      stroke-width="2.5"
    />
    <circle
      cx="42"
      cy="33"
      r="7"
      fill="none"
      stroke="#3B2A22"
      stroke-width="2.5"
    />
    <path d="M33 33h2" stroke="#3B2A22" stroke-width="2.5" />
  {/if}
</svg>

<style>
  svg {
    flex: none;
    filter: drop-shadow(0 1px 1px rgba(30, 40, 40, 0.28));
  }

  .buddy-breathe {
    transform-box: fill-box;
    transform-origin: center;
    animation: buddy-breathe 2.6s ease-in-out infinite;
  }

  @keyframes buddy-breathe {
    50% {
      transform: scale(1.08);
    }
  }
</style>
