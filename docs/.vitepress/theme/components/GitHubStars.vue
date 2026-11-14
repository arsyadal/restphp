<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'

const stars = ref<number | null>(null)
const unavailable = ref(false)
const formatter = new Intl.NumberFormat('en-US')
let interval: ReturnType<typeof setInterval> | undefined
let activeRequest: AbortController | undefined

const countLabel = computed(() => {
  if (unavailable.value) return 'Unavailable'
  if (stars.value === null) return 'Loading…'
  return `${formatter.format(stars.value)} ${stars.value === 1 ? 'star' : 'stars'}`
})
const displayCount = computed(() => unavailable.value
  ? '—'
  : stars.value === null ? '…' : formatter.format(stars.value))
const status = computed(() => {
  if (unavailable.value) return 'GitHub star count is unavailable.'
  return stars.value === null
    ? 'Checking GitHub…'
    : 'Updates from GitHub every 60s while this tab is visible.'
})

async function refresh() {
  if (activeRequest) return
  const request = new AbortController()
  activeRequest = request
  const timeout = setTimeout(() => request.abort(), 10_000)
  try {
    const response = await fetch('https://api.github.com/repos/arsyadal/restphp', {
      headers: { Accept: 'application/vnd.github+json' },
      cache: 'no-cache',
      signal: request.signal,
    })
    if (!response.ok) throw new Error('GitHub request failed')
    const repository = await response.json()
    if (!Number.isSafeInteger(repository.stargazers_count) || repository.stargazers_count < 0) {
      throw new Error('GitHub returned an invalid star count')
    }
    stars.value = repository.stargazers_count
    unavailable.value = false
  } catch {
    unavailable.value = true
  } finally {
    clearTimeout(timeout)
    activeRequest = undefined
  }
}

onMounted(() => {
  void refresh()
  interval = setInterval(() => {
    if (!document.hidden) void refresh()
  }, 60_000)
})

onUnmounted(() => {
  clearInterval(interval)
  activeRequest?.abort()
})
</script>

<template>
  <a
    class="github-star-link"
    href="https://github.com/arsyadal/restphp"
    target="_blank"
    rel="noopener noreferrer"
    :aria-label="`Give RestPHP a star on GitHub (opens in a new tab). ${countLabel}.`"
    :title="`Give a star · ${countLabel}. ${status}`"
  >
    <svg class="github-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path fill="currentColor" d="M12 .297c-6.63 0-12 5.373-12 12c0 5.303 3.438 9.8 8.205 11.385c.6.113.82-.258.82-.577c0-.285-.01-1.04-.015-2.04c-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729c1.205.084 1.838 1.236 1.838 1.236c1.07 1.835 2.809 1.305 3.495.998c.108-.776.417-1.305.76-1.605c-2.665-.3-5.466-1.332-5.466-5.93c0-1.31.465-2.38 1.235-3.22c-.135-.303-.54-1.523.105-3.176c0 0 1.005-.322 3.3 1.23c.96-.267 1.98-.399 3-.405c1.02.006 2.04.138 3 .405c2.28-1.552 3.285-1.23 3.285-1.23c.645 1.653.24 2.873.12 3.176c.765.84 1.23 1.91 1.23 3.22c0 4.61-2.805 5.625-5.475 5.92c.42.36.81 1.096.81 2.22c0 1.606-.015 2.896-.015 3.286c0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/>
    </svg>
    <span class="star-prompt">Give a star</span>
    <span class="star-count" aria-live="polite" aria-atomic="true">{{ displayCount }}</span>
    <span class="sr-only" role="status">{{ status }}</span>
  </a>
</template>

<style scoped>
.github-star-link {
  display: inline-flex;
  flex-shrink: 0;
  align-items: center;
  gap: 8px;
  min-height: 44px;
  padding: 0 8px;
  color: var(--vp-c-text-2);
  font-size: 14px;
  font-weight: 500;
  text-decoration: none;
  white-space: nowrap;
}

.github-icon {
  width: 20px;
  height: 20px;
}

.star-prompt {
  display: none;
}

.github-star-link:hover,
.github-star-link:active {
  color: var(--vp-c-brand-1);
}

.github-star-link:focus-visible {
  outline: 2px solid var(--vp-c-brand-1);
  outline-offset: 2px;
}

.star-count {
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

/* The full navigation and GitHub count do not fit at the theme's 768px breakpoint. */
@media (min-width: 768px) and (max-width: 1279px) {
  :global(.VPNavBar .VPNavBarMenu),
  :global(.VPNavBar .VPNavBarExtra) {
    display: none;
  }

  :global(.VPNavBar .VPNavBarHamburger) {
    display: flex;
  }

  :global(.VPNav .VPNavScreen) {
    display: block;
  }
}

@media (min-width: 1280px) {
  .star-prompt {
    display: inline;
  }
}
</style>
