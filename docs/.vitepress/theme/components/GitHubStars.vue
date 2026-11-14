<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { Star } from 'lucide-vue-next'

const stars = ref<number | null>(null)
const unavailable = ref(false)
const formatter = new Intl.NumberFormat('en-US')
let interval: ReturnType<typeof setInterval> | undefined
let activeRequest: AbortController | undefined
let starRequest: AbortController | undefined
const username = ref('')
const checkedUsername = ref('')
const starred = ref<boolean | null>(null)
const checkingStar = ref(false)
const checkMessage = ref('')
const starLabel = computed(() => starred.value === true ? 'Starred' : 'Give a star')
const starDescription = computed(() => starred.value === true
  ? `Starred by @${checkedUsername.value}`
  : 'Check a GitHub username or open the repository to give a star')

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

async function checkStar(name = username.value.trim().replace(/^@/, '')) {
  if (!name || checkingStar.value) return
  checkingStar.value = true
  starred.value = null
  checkedUsername.value = name
  checkMessage.value = `Checking public star status for @${name}…`
  const request = new AbortController()
  starRequest = request
  const timeout = setTimeout(() => request.abort(), 10_000)
  try {
    const login = name.toLowerCase()
    for (let page = 1; ; page++) {
      const response = await fetch(
        `https://api.github.com/repos/arsyadal/restphp/stargazers?per_page=100&page=${page}`,
        {
          headers: { Accept: 'application/vnd.github+json' },
          cache: 'no-cache',
          signal: request.signal,
        },
      )
      if (!response.ok) throw new Error('GitHub request failed')
      const users = await response.json()
      if (!Array.isArray(users) || users.some(user => typeof user?.login !== 'string')) {
        throw new Error('GitHub returned an invalid stargazer list')
      }
      if (users.some(user => user.login.toLowerCase() === login)) {
        starred.value = true
        break
      }
      if (users.length < 100) {
        starred.value = false
        break
      }
    }
    checkMessage.value = starred.value
      ? `@${name} has starred RestPHP.`
      : `@${name} is not in RestPHP's public stargazer list.`
    try {
      localStorage.setItem('restphp.github-username', name)
    } catch {
      checkMessage.value += ' This browser could not remember the username.'
    }
  } catch {
    starred.value = null
    checkMessage.value = 'Star status is unavailable. Try again later.'
  } finally {
    clearTimeout(timeout)
    starRequest = undefined
    checkingStar.value = false
  }
}

function forgetUsername() {
  username.value = ''
  checkedUsername.value = ''
  starred.value = null
  checkMessage.value = ''
  try {
    localStorage.removeItem('restphp.github-username')
  } catch {
    checkMessage.value = 'This browser could not clear the remembered username.'
  }
}

function onFocus() {
  void refresh()
  if (checkedUsername.value) void checkStar(checkedUsername.value)
}

onMounted(() => {
  void refresh()
  try {
    username.value = localStorage.getItem('restphp.github-username') || ''
    if (username.value) void checkStar()
  } catch {
    checkMessage.value = 'Browser storage is unavailable; you can still check a username.'
  }
  interval = setInterval(() => {
    if (!document.hidden) void refresh()
  }, 60_000)
  window.addEventListener('focus', onFocus)
})

onUnmounted(() => {
  clearInterval(interval)
  activeRequest?.abort()
  starRequest?.abort()
  window.removeEventListener('focus', onFocus)
})
</script>

<template>
  <div class="github-stars">
    <button
      class="github-star-link"
      :class="{ 'is-starred': starred === true }"
      type="button"
      popovertarget="github-star-panel"
      :aria-label="`${starDescription}. ${countLabel}.`"
      :title="`${starDescription}. ${countLabel}. ${status}`"
    >
      <Star class="star-icon" :size="20" aria-hidden="true" />
      <span class="star-prompt">{{ starLabel }}</span>
      <span class="star-count" aria-live="polite" aria-atomic="true">{{ displayCount }}</span>
      <span class="sr-only" role="status">{{ status }}</span>
    </button>
    <div id="github-star-panel" class="star-panel" popover role="dialog" aria-labelledby="star-panel-title">
      <div class="panel-heading">
        <h2 id="star-panel-title">Star RestPHP</h2>
        <button class="panel-button" type="button" popovertarget="github-star-panel" popovertargetaction="hide">Close</button>
      </div>
      <a class="repo-action" href="https://github.com/arsyadal/restphp" target="_blank" rel="noopener noreferrer">
        Open repository on GitHub
      </a>
      <p class="panel-help">Already starred? Check a public GitHub username below. This is not a GitHub login; only the username is remembered on this browser.</p>
      <form @submit.prevent="checkStar()">
        <label for="star-username">GitHub username</label>
        <div class="check-row">
          <input id="star-username" v-model="username" name="github-username" autocomplete="username" autocapitalize="none" :spellcheck="false" maxlength="39" required :disabled="checkingStar">
          <button class="panel-button" type="submit" :disabled="checkingStar">{{ checkingStar ? 'Checking…' : 'Check' }}</button>
        </div>
      </form>
      <p v-if="checkMessage" class="check-message" role="status">{{ checkMessage }}</p>
      <button v-if="checkedUsername" class="panel-button forget-username" type="button" :disabled="checkingStar" @click="forgetUsername">Forget username</button>
    </div>
  </div>
</template>

<style scoped>
.github-star-link {
  display: inline-flex;
  flex-shrink: 0;
  align-items: center;
  gap: 8px;
  min-height: 44px;
  padding: 0 8px;
  border: 0;
  background: transparent;
  cursor: pointer;
  font-family: inherit;
  color: var(--vp-c-text-2);
  font-size: 14px;
  font-weight: 500;
  white-space: nowrap;
}

.star-icon {
  width: 20px;
  height: 20px;
}

.is-starred .star-icon {
  color: #9a6700;
  fill: #eac54f;
}

:global(.dark) .is-starred .star-icon {
  color: #e3b341;
  fill: currentColor;
}

.star-prompt {
  display: none;
}

.github-star-link:hover,
.github-star-link:active {
  color: var(--vp-c-brand-1);
}

.github-star-link:focus-visible,
.star-panel a:focus-visible,
.star-panel button:focus-visible,
.star-panel input:focus-visible {
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

.star-panel {
  position: fixed;
  inset: auto 16px auto auto;
  top: calc(var(--vp-nav-height) + var(--vp-layout-top-height, 0px) + 8px);
  width: min(22rem, calc(100vw - 32px));
  max-height: calc(100dvh - var(--vp-nav-height) - 24px);
  overflow: auto;
  padding: 16px;
  border: 1px solid var(--vp-c-text-2);
  border-radius: 8px;
  background: var(--vp-c-bg);
  color: var(--vp-c-text-1);
  font-size: 14px;
  line-height: 1.5;
}

.panel-heading,
.check-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.panel-heading {
  justify-content: space-between;
}

.panel-heading h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.panel-button,
.repo-action {
  min-height: 44px;
  padding: 8px 12px;
  border: 1px solid var(--vp-c-text-2);
  border-radius: 6px;
  background: var(--vp-c-default-soft);
  color: var(--vp-c-text-1);
  font: inherit;
  cursor: pointer;
}

.panel-button:disabled {
  cursor: wait;
}

.repo-action {
  display: flex;
  align-items: center;
  justify-content: center;
  margin-top: 12px;
}

.repo-action:hover {
  text-decoration: underline;
}

.panel-help,
.check-message {
  margin: 12px 0;
}

.star-panel label {
  display: block;
  margin-bottom: 6px;
  font-weight: 600;
}

.check-row input {
  flex: 1;
  min-width: 0;
  min-height: 44px;
  padding: 8px;
  border: 1px solid var(--vp-c-text-2);
  border-radius: 6px;
  background: var(--vp-c-bg);
  color: var(--vp-c-text-1);
  font: inherit;
}

.forget-username {
  margin-top: 8px;
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
