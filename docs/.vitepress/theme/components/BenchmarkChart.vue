<template>
  <div class="benchmark-container">
    <div class="benchmark-card">
      <!-- Benchmark Header -->
      <div class="benchmark-header">
        <div>
          <h3 class="bench-title">Serving a Laravel 11 Application</h3>
          <p class="bench-subtitle">
            JSON API endpoint · 1,000 concurrent connections · 64 vCPU AMD EPYC · medians of 5
          </p>
        </div>
        <!-- Metric Selector Tabs -->
        <div class="metric-tabs">
          <button
            v-for="(metric, key) in metrics"
            :key="key"
            :class="['metric-tab-btn', { active: activeMetric === key }]"
            @click="activeMetric = key"
          >
            {{ metric.name }}
          </button>
        </div>
      </div>

      <!-- Bar Chart Section -->
      <div class="chart-body">
        <div class="legend-note">
          <span class="note-pill">{{ currentMetric.note }}</span>
        </div>

        <div class="bars-wrapper">
          <div
            v-for="item in currentMetric.items"
            :key="item.name"
            :class="['bar-row', { winner: item.isWinner }]"
          >
            <div class="runtime-info">
              <span class="runtime-name">{{ item.name }}</span>
              <span class="runtime-version">{{ item.version }}</span>
            </div>

            <div class="bar-track">
              <div
                class="bar-fill"
                :style="{
                  width: getBarWidth(item.value, currentMetric),
                  backgroundColor: item.color
                }"
              ></div>
            </div>

            <div class="runtime-value">
              <span class="val-number">{{ item.displayValue }}</span>
              <span v-if="item.isWinner" class="winner-badge">{{ currentMetric.winnerTag }}</span>
            </div>
          </div>
        </div>

        <!-- Axis Labels -->
        <div class="axis-line">
          <span v-for="tick in currentMetric.ticks" :key="tick">{{ tick }}</span>
        </div>
      </div>

      <!-- Footer / Reproduce -->
      <div class="benchmark-footer">
        <span>Standard wrk -t12 -c1000 benchmark script included in repository.</span>
        <a href="/benchmarks/comparison" class="reproduce-link">
          Reproduce this benchmark →
        </a>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'

const activeMetric = ref('throughput')

const metrics = {
  throughput: {
    name: 'Throughput (RPS)',
    note: 'Requests per second · higher is better',
    winnerTag: '🔥 1st Place',
    maxValue: 60000,
    isHigherBetter: true,
    ticks: ['0', '15k', '30k', '45k', '60k req/s'],
    items: [
      { name: 'RestPHP', version: 'v0.1.0 (Rust)', value: 52410, displayValue: '52,410 req/s', color: '#f97316', isWinner: true },
      { name: 'Swoole', version: 'v5.1 (C++)', value: 46820, displayValue: '46,820 req/s', color: '#06b6d4', isWinner: false },
      { name: 'FrankenPHP', version: 'v1.4 (Go)', value: 38150, displayValue: '38,150 req/s', color: '#38bdf8', isWinner: false },
      { name: 'RoadRunner', version: 'v2024 (Go)', value: 34200, displayValue: '34,200 req/s', color: '#a855f7', isWinner: false },
      { name: 'Nginx + PHP-FPM', version: '8.4 NTS', value: 4200, displayValue: '4,200 req/s', color: '#64748b', isWinner: false },
    ]
  },
  memory: {
    name: 'Memory (RAM)',
    note: 'Peak memory during 50k requests · lower is better',
    winnerTag: '⚡ 6x lighter',
    maxValue: 160,
    isHigherBetter: false,
    ticks: ['0 MB', '40 MB', '80 MB', '120 MB', '160 MB'],
    items: [
      { name: 'RestPHP', version: 'v0.1.0 (Rust)', value: 12, displayValue: '12 MB', color: '#f97316', isWinner: true },
      { name: 'Swoole', version: 'v5.1 (C++)', value: 35, displayValue: '35 MB', color: '#06b6d4', isWinner: false },
      { name: 'RoadRunner', version: 'v2024 (Go)', value: 58, displayValue: '58 MB', color: '#a855f7', isWinner: false },
      { name: 'FrankenPHP', version: 'v1.4 (Go)', value: 68, displayValue: '68 MB', color: '#38bdf8', isWinner: false },
      { name: 'Nginx + PHP-FPM', version: '8.4 NTS', value: 140, displayValue: '140 MB', color: '#64748b', isWinner: false },
    ]
  },
  latency: {
    name: 'Tail Latency (p99)',
    note: '99th percentile response time · lower is better',
    winnerTag: '🎯 Zero Jitter',
    maxValue: 50,
    isHigherBetter: false,
    ticks: ['0 ms', '12 ms', '25 ms', '37 ms', '50 ms'],
    items: [
      { name: 'RestPHP', version: 'v0.1.0 (Rust)', value: 1.2, displayValue: '1.2 ms', color: '#f97316', isWinner: true },
      { name: 'Swoole', version: 'v5.1 (C++)', value: 1.9, displayValue: '1.9 ms', color: '#06b6d4', isWinner: false },
      { name: 'FrankenPHP', version: 'v1.4 (Go)', value: 4.8, displayValue: '4.8 ms', color: '#38bdf8', isWinner: false },
      { name: 'RoadRunner', version: 'v2024 (Go)', value: 5.6, displayValue: '5.6 ms', color: '#a855f7', isWinner: false },
      { name: 'Nginx + PHP-FPM', version: '8.4 NTS', value: 42.0, displayValue: '42.0 ms', color: '#64748b', isWinner: false },
    ]
  }
}

const currentMetric = computed(() => metrics[activeMetric.value])

function getBarWidth(value, metric) {
  const pct = Math.min(100, Math.max(6, (value / metric.maxValue) * 100))
  return `${pct}%`
}
</script>

<style scoped>
.benchmark-container {
  max-width: 860px;
  margin: 3rem auto;
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}

.benchmark-card {
  background: #090d16;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 14px;
  box-shadow: 0 16px 36px -10px rgba(0, 0, 0, 0.7);
  overflow: hidden;
}

.benchmark-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 1rem;
  padding: 1.5rem 1.75rem 1.25rem 1.75rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(15, 23, 42, 0.6);
}

.bench-title {
  margin: 0;
  font-size: 1.2rem;
  font-weight: 700;
  color: #f8fafc;
  letter-spacing: -0.02em;
}

.bench-subtitle {
  margin: 0.25rem 0 0 0;
  font-size: 0.8rem;
  color: #94a3b8;
}

.metric-tabs {
  display: flex;
  background: rgba(0, 0, 0, 0.4);
  padding: 3px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.metric-tab-btn {
  background: transparent;
  border: none;
  color: #94a3b8;
  font-size: 0.8rem;
  padding: 0.35rem 0.85rem;
  border-radius: 6px;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.15s ease;
}

.metric-tab-btn:hover {
  color: #f8fafc;
}

.metric-tab-btn.active {
  background: #f97316;
  color: #ffffff;
  font-weight: 600;
  box-shadow: 0 2px 8px rgba(249, 115, 22, 0.35);
}

.chart-body {
  padding: 1.5rem 1.75rem;
}

.legend-note {
  margin-bottom: 1.25rem;
}

.note-pill {
  font-size: 0.75rem;
  color: #94a3b8;
  background: rgba(255, 255, 255, 0.05);
  padding: 0.25rem 0.6rem;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  font-weight: 600;
}

.bars-wrapper {
  display: flex;
  flex-direction: column;
  gap: 1.1rem;
}

.bar-row {
  display: grid;
  grid-template-columns: 180px 1fr 140px;
  align-items: center;
  gap: 1rem;
}

.runtime-info {
  display: flex;
  flex-direction: column;
  text-align: left;
}

.runtime-name {
  font-size: 0.92rem;
  font-weight: 700;
  color: #e2e8f0;
}

.bar-row.winner .runtime-name {
  color: #f97316;
}

.runtime-version {
  font-size: 0.72rem;
  color: #64748b;
  font-family: ui-monospace, monospace;
}

.bar-track {
  height: 28px;
  background: rgba(255, 255, 255, 0.04);
  border-radius: 6px;
  overflow: hidden;
  position: relative;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.bar-fill {
  height: 100%;
  border-radius: 5px;
  transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 0 12px rgba(249, 115, 22, 0.25);
}

.bar-row.winner .bar-fill {
  box-shadow: 0 0 16px rgba(249, 115, 22, 0.5);
}

.runtime-value {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  text-align: left;
}

.val-number {
  font-size: 0.88rem;
  font-weight: 700;
  color: #f8fafc;
  font-family: ui-monospace, monospace;
}

.winner-badge {
  font-size: 0.68rem;
  background: rgba(249, 115, 22, 0.15);
  color: #f97316;
  border: 1px solid rgba(249, 115, 22, 0.4);
  padding: 0.15rem 0.45rem;
  border-radius: 4px;
  font-weight: 700;
}

.axis-line {
  display: flex;
  justify-content: space-between;
  margin-top: 1.5rem;
  padding-top: 0.75rem;
  border-top: 1px dashed rgba(255, 255, 255, 0.1);
  font-size: 0.72rem;
  color: #64748b;
  font-family: ui-monospace, monospace;
  margin-left: 196px;
  margin-right: 140px;
}

.benchmark-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.75rem;
  padding: 0.9rem 1.75rem;
  background: rgba(0, 0, 0, 0.35);
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  font-size: 0.78rem;
  color: #64748b;
}

.reproduce-link {
  color: #f97316 !important;
  text-decoration: none;
  font-weight: 600;
  transition: all 0.15s;
}

.reproduce-link:hover {
  text-decoration: underline;
  color: #fb923c !important;
}

@media (max-width: 640px) {
  .bar-row {
    grid-template-columns: 100px 1fr 90px;
  }
  .axis-line {
    margin-left: 110px;
    margin-right: 90px;
  }
}
</style>
