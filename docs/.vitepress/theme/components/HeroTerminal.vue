<template>
  <div class="terminal-wrapper">
    <div class="terminal-glow"></div>
    <div class="terminal-card">
      <!-- Terminal Header Bar -->
      <div class="terminal-header">
        <div class="traffic-lights">
          <span class="dot red"></span>
          <span class="dot yellow"></span>
          <span class="dot green"></span>
        </div>
        <div class="tab-list">
          <button
            v-for="(tab, index) in tabs"
            :key="tab.id"
            :class="['tab-btn', { active: activeTab === index }]"
            @click="activeTab = index"
          >
            {{ tab.name }}
          </button>
        </div>
        <button class="copy-btn" @click="copyCommand" :title="copied ? 'Copied!' : 'Copy to clipboard'">
          <svg v-if="!copied" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
          <svg v-else xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#22c55e" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
          <span>{{ copied ? 'Copied' : 'Copy' }}</span>
        </button>
      </div>

      <!-- Terminal Body -->
      <div class="terminal-body">
        <div class="command-line">
          <span class="prompt">$</span>
          <span class="command">{{ currentTab.command }}</span>
        </div>
        <div class="terminal-output" v-html="currentTab.outputHtml"></div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'

const activeTab = ref(0)
const copied = ref(false)

const tabs = [
  {
    id: 'install',
    name: '1. Install',
    command: 'curl -fsSL https://restphp.dev/install.sh | bash',
    outputHtml: `<span class="dim">info</span> Downloading restphp v0.1.0 (x86_64-unknown-linux-gnu)...<br/>
<span class="success">✓</span> Verified binary signature & checksums<br/>
<span class="success">✓</span> Installed executable to <span class="highlight">/usr/local/bin/restphp</span><br/>
<span class="muted">Run "restphp --help" to explore commands.</span>`
  },
  {
    id: 'serve',
    name: '2. Run Any Script',
    command: 'restphp app.php',
    outputHtml: `<span class="brand">RestPHP v0.1.0</span> <span class="muted">(standalone async server)</span><br/><br/>
  <span class="dim">➜</span>  <span class="label">Local:</span>   <span class="highlight">http://localhost:8080/</span><br/>
  <span class="dim">➜</span>  <span class="label">Serving:</span> <span class="success">app.php</span><br/><br/>
<span class="dim">Ready in 1.4ms (p99 tail latency: 1.2ms)</span>`
  },
  {
    id: 'laravel',
    name: '3. Run Laravel',
    command: 'restphp',
    outputHtml: `<span class="dim">✨</span> <span class="highlight">Detected Laravel application (artisan found)</span><br/>
<span class="success">✓</span> Running directly on RestPHP Standalone Engine<br/>
  <span class="dim">➜</span>  <span class="label">Local:</span>   <span class="highlight">http://localhost:8000/</span><br/>
  <span class="dim">➜</span>  <span class="label">Serving:</span> <span class="success">public/index.php</span> <span class="muted">(Persistent Worker)</span><br/><br/>
<span class="brand">🔥 52,400+ req/s</span> <span class="dim">· Zero Host GC · Zero configuration</span>`
  },
  {
    id: 'eval',
    name: '4. Quick Eval',
    command: 'restphp -e \'echo "PHP Version: " . PHP_VERSION . "\\n";\'',
    outputHtml: `<span class="success">PHP Version: 8.4.24</span><br/>
<span class="dim">Execution completed in memory in 0.7ms</span>`
  }
]

const currentTab = computed(() => tabs[activeTab.value])

const copyCommand = async () => {
  try {
    await navigator.clipboard.writeText(currentTab.value.command)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch (err) {
    console.error('Failed to copy: ', err)
  }
}
</script>

<style scoped>
.terminal-wrapper {
  position: relative;
  max-width: 860px;
  margin: 1.5rem auto 3rem auto;
}

.terminal-glow {
  position: absolute;
  top: -10px;
  left: 5%;
  right: 5%;
  height: 120%;
  background: radial-gradient(ellipse at top, rgba(249, 115, 22, 0.22), transparent 70%);
  filter: blur(28px);
  pointer-events: none;
  z-index: 0;
}

.terminal-card {
  position: relative;
  z-index: 1;
  background: #090d16;
  border: 1px solid rgba(249, 115, 22, 0.35);
  box-shadow: 0 20px 40px -15px rgba(0, 0, 0, 0.8), 0 0 20px rgba(249, 115, 22, 0.15);
  border-radius: 14px;
  overflow: hidden;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
}

.terminal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.65rem 1rem;
  background: rgba(15, 23, 42, 0.85);
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.traffic-lights {
  display: flex;
  gap: 7px;
  align-items: center;
}

.dot {
  width: 11px;
  height: 11px;
  border-radius: 50%;
  display: inline-block;
}

.dot.red { background: #ef4444; }
.dot.yellow { background: #f59e0b; }
.dot.green { background: #10b981; }

.tab-list {
  display: flex;
  gap: 4px;
  background: rgba(0, 0, 0, 0.35);
  padding: 3px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.tab-btn {
  background: transparent;
  border: none;
  color: #94a3b8;
  font-size: 0.78rem;
  padding: 0.3rem 0.75rem;
  border-radius: 6px;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.15s ease;
}

.tab-btn:hover {
  color: #f8fafc;
}

.tab-btn.active {
  background: #f97316;
  color: #ffffff;
  font-weight: 600;
  box-shadow: 0 2px 8px rgba(249, 115, 22, 0.35);
}

.copy-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #cbd5e1;
  font-size: 0.75rem;
  padding: 0.3rem 0.65rem;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.copy-btn:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #ffffff;
  border-color: #f97316;
}

.terminal-body {
  padding: 1.25rem 1.5rem;
  font-size: 0.88rem;
  line-height: 1.6;
  color: #e2e8f0;
  text-align: left;
}

.command-line {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding-bottom: 0.9rem;
  margin-bottom: 0.9rem;
  border-bottom: 1px dashed rgba(255, 255, 255, 0.1);
  font-weight: 600;
}

.prompt {
  color: #f97316;
  user-select: none;
  font-weight: 800;
}

.command {
  color: #f8fafc;
}

.terminal-output {
  color: #94a3b8;
  font-size: 0.82rem;
}

:deep(.highlight) {
  color: #38bdf8;
}

:deep(.success) {
  color: #22c55e;
  font-weight: 600;
}

:deep(.brand) {
  color: #f97316;
  font-weight: 700;
}

:deep(.dim) {
  color: #64748b;
}

:deep(.muted) {
  color: #475569;
}

:deep(.label) {
  color: #cbd5e1;
  font-weight: 600;
}

@media (max-width: 640px) {
  .tab-list {
    display: none;
  }
}
</style>
