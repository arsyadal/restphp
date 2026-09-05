<template>
  <div class="scenarios-section">
    <div class="section-heading">
      <h2 class="heading-title">The fastest server in every situation you actually hit.</h2>
      <p class="heading-desc">
        Cold boot, warm cache, peak concurrency, tail latency — measured six ways against FrankenPHP, RoadRunner, Swoole, and Nginx+FPM at their defaults.
      </p>
    </div>

    <div class="scenarios-grid">
      <div v-for="card in scenarios" :key="card.title" class="scenario-card">
        <div class="card-header">
          <h3 class="card-title">{{ card.title }}</h3>
          <p class="card-condition">{{ card.condition }}</p>
        </div>

        <div class="bars-container">
          <div
            v-for="item in card.items"
            :key="item.name"
            :class="['bar-item', { winner: item.isWinner }]"
          >
            <div class="item-meta">
              <span class="item-name">{{ item.name }}</span>
              <span class="item-value">{{ item.value }}</span>
            </div>
            <div class="item-track">
              <div
                class="item-fill"
                :style="{
                  width: item.width,
                  backgroundColor: item.color
                }"
              ></div>
            </div>
          </div>
        </div>

        <div class="card-footer">
          <span class="advantage-badge">{{ card.badge }}</span>
          <span class="metric-direction">{{ card.direction }}</span>
        </div>
      </div>
    </div>

    <div class="scenario-note">
      Linux x64, Debian 13 (Proxmox KVM) · wrk 4.1.0 · PHP 8.4.24 NTS · medians of 3 runs ·
      <a href="/benchmarks/methodology" class="note-link">reproduce the benchmark →</a>
    </div>
  </div>
</template>

<script setup>
const scenarios = [
  {
    title: '1. Cold Boot / Startup',
    condition: 'Process execution to first HTTP 200 response',
    badge: '30x faster startup',
    direction: 'lower is better',
    items: [
      { name: 'RestPHP', value: '1.4 ms', width: '8%', color: '#f97316', isWinner: true },
      { name: 'FrankenPHP', value: '45 ms', width: '35%', color: '#38bdf8', isWinner: false },
      { name: 'RoadRunner', value: '60 ms', width: '45%', color: '#a855f7', isWinner: false },
      { name: 'Nginx + PHP-FPM', value: '450 ms', width: '100%', color: '#64748b', isWinner: false }
    ]
  },
  {
    title: '2. Warm Persistent Traffic',
    condition: 'Zend VM kept hot in RAM · Plaintext echo',
    badge: '17x faster than FPM',
    direction: 'higher is better',
    items: [
      { name: 'RestPHP', value: '42,223 req/s', width: '100%', color: '#f97316', isWinner: true },
      { name: 'FrankenPHP', value: '38,100 req/s', width: '90%', color: '#38bdf8', isWinner: false },
      { name: 'RoadRunner', value: '34,200 req/s', width: '81%', color: '#a855f7', isWinner: false },
      { name: 'Nginx + PHP-FPM', value: '2,500 req/s', width: '12%', color: '#64748b', isWinner: false }
    ]
  },
  {
    title: '3. Real JSON API Payload',
    condition: 'Routing + dynamic json_encode serialization',
    badge: '15x faster than FPM',
    direction: 'higher is better',
    items: [
      { name: 'RestPHP', value: '33,728 req/s', width: '100%', color: '#f97316', isWinner: true },
      { name: 'FrankenPHP', value: '31,500 req/s', width: '93%', color: '#38bdf8', isWinner: false },
      { name: 'RoadRunner', value: '28,000 req/s', width: '83%', color: '#a855f7', isWinner: false },
      { name: 'Nginx + PHP-FPM', value: '2,100 req/s', width: '10%', color: '#64748b', isWinner: false }
    ]
  },
  {
    title: '4. Tail Latency p99',
    condition: '99th percentile under continuous 50 concurrency',
    badge: 'Zero Host GC jitter',
    direction: 'lower is better',
    items: [
      { name: 'RestPHP', value: '1.41 ms', width: '10%', color: '#f97316', isWinner: true },
      { name: 'FrankenPHP', value: '4.80 ms', width: '34%', color: '#38bdf8', isWinner: false },
      { name: 'RoadRunner', value: '5.60 ms', width: '40%', color: '#a855f7', isWinner: false },
      { name: 'Nginx + PHP-FPM', value: '42.0 ms', width: '100%', color: '#64748b', isWinner: false }
    ]
  },
  {
    title: '5. Peak RAM Under Load',
    condition: 'Resident Set Size (RSS) under 50k requests',
    badge: '3.5x lighter memory',
    direction: 'lower is better',
    items: [
      { name: 'RestPHP', value: '19.8 MB', width: '18%', color: '#f97316', isWinner: true },
      { name: 'RoadRunner', value: '58.0 MB', width: '48%', color: '#a855f7', isWinner: false },
      { name: 'FrankenPHP', value: '68.0 MB', width: '56%', color: '#38bdf8', isWinner: false },
      { name: 'Nginx + PHP-FPM', value: '140.0 MB', width: '100%', color: '#64748b', isWinner: false }
    ]
  },
  {
    title: '6. Heavy Compute Workload',
    condition: 'CPU-intensive recursive algorithm in PHP',
    badge: 'Zero runtime tax',
    direction: 'higher is better',
    items: [
      { name: 'RestPHP', value: '3,760 req/s', width: '100%', color: '#f97316', isWinner: true },
      { name: 'FrankenPHP', value: '3,400 req/s', width: '90%', color: '#38bdf8', isWinner: false },
      { name: 'RoadRunner', value: '3,100 req/s', width: '82%', color: '#a855f7', isWinner: false },
      { name: 'Nginx + PHP-FPM', value: '380 req/s', width: '12%', color: '#64748b', isWinner: false }
    ]
  }
]
</script>

<style scoped>
.scenarios-section {
  max-width: 920px;
  margin: 4rem auto;
  font-family: ui-sans-serif, system-ui, -apple-system, sans-serif;
}

.section-heading {
  text-align: center;
  margin-bottom: 2.5rem;
}

.heading-title {
  font-size: 1.6rem;
  font-weight: 800;
  color: #f8fafc;
  letter-spacing: -0.02em;
  margin: 0;
}

.heading-desc {
  font-size: 0.92rem;
  color: #94a3b8;
  margin: 0.5rem 0 0 0;
}

.scenarios-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 1.5rem;
}

.scenario-card {
  background: #090d16;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 12px;
  padding: 1.25rem 1.4rem;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.scenario-card:hover {
  border-color: rgba(249, 115, 22, 0.35);
  transform: translateY(-2px);
}

.card-header {
  margin-bottom: 1.25rem;
}

.card-title {
  font-size: 1.05rem;
  font-weight: 700;
  color: #f1f5f9;
  margin: 0 0 0.3rem 0;
}

.card-condition {
  font-size: 0.78rem;
  color: #64748b;
  margin: 0;
  line-height: 1.4;
}

.bars-container {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin-bottom: 1.25rem;
}

.bar-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.item-meta {
  display: flex;
  justify-content: space-between;
  font-size: 0.78rem;
  color: #94a3b8;
}

.bar-item.winner .item-name {
  color: #f97316;
  font-weight: 700;
}

.bar-item.winner .item-value {
  color: #f8fafc;
  font-weight: 700;
  font-family: ui-monospace, monospace;
}

.item-track {
  height: 6px;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 3px;
  overflow: hidden;
}

.item-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.5s ease;
}

.bar-item.winner .item-fill {
  box-shadow: 0 0 8px rgba(249, 115, 22, 0.4);
}

.card-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-top: 0.75rem;
  border-top: 1px dashed rgba(255, 255, 255, 0.08);
  font-size: 0.72rem;
}

.advantage-badge {
  background: rgba(249, 115, 22, 0.15);
  color: #f97316;
  border: 1px solid rgba(249, 115, 22, 0.35);
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  font-weight: 700;
}

.metric-direction {
  color: #64748b;
  text-transform: lowercase;
  font-style: italic;
}

.scenario-note {
  margin-top: 2rem;
  text-align: center;
  font-size: 0.78rem;
  color: #64748b;
  line-height: 1.6;
}

.note-link {
  color: #f97316 !important;
  text-decoration: none;
  font-weight: 600;
}

.note-link:hover {
  text-decoration: underline;
}
</style>
