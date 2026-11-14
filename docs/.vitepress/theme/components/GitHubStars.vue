<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { Star } from 'lucide-vue-next'

const stars = ref<number | null>(null)
const unavailable = ref(false)
const formatter = new Intl.NumberFormat('en-US')
let interval: ReturnType<typeof setInterval> | undefined
let activeRequest: AbortController | undefined

const countLabel = computed(() => {
  if (stars.value === null) return unavailable.value ? 'Unavailable' : 'Loading…'
  return `${formatter.format(stars.value)} ${stars.value === 1 ? 'star' : 'stars'}`
})
const status = computed(() => {
  if (unavailable.value) {
    return stars.value === null
      ? 'GitHub star count is unavailable.'
      : 'Last known count; GitHub could not be refreshed.'
  }
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
  <div class="github-stars">
    <a
      class="github-star-link"
      href="https://github.com/arsyadal/restphp"
      target="_blank"
      rel="noopener noreferrer"
      aria-label="Give RestPHP a star on GitHub (opens in a new tab)"
    >
      <Star :size="18" aria-hidden="true" />
      <span>Give a star</span>
      <span class="star-count" aria-live="polite" aria-atomic="true">{{ countLabel }}</span>
    </a>
    <p class="star-status" role="status">{{ status }}</p>
  </div>
</template>

<style scoped>
.github-stars {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  margin-top: 16px;
}

.github-star-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  min-height: 44px;
  max-width: 100%;
  padding: 8px 16px;
  border: 1px solid var(--vp-c-text-2);
  border-radius: 8px;
  background: var(--vp-c-bg);
  color: var(--vp-c-text-1);
  font-size: 14px;
  font-weight: 600;
  text-decoration: none;
}

.github-star-link:hover,
.github-star-link:active {
  border-color: var(--vp-c-brand-1);
  background: var(--vp-c-default-soft);
  color: var(--vp-c-brand-1);
}

.github-star-link:focus-visible {
  outline: 2px solid var(--vp-c-brand-1);
  outline-offset: 3px;
}

.star-count {
  padding-inline-start: 10px;
  border-inline-start: 1px solid var(--vp-c-divider);
  font-variant-numeric: tabular-nums;
}

.star-status {
  margin: 0;
  color: var(--vp-c-text-2);
  font-size: 12px;
  line-height: 1.5;
}

@media (min-width: 960px) {
  .github-stars {
    align-items: flex-start;
  }
}
</style>
