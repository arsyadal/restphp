import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import HeroTerminal from './components/HeroTerminal.vue'
import BenchmarkChart from './components/BenchmarkChart.vue'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('HeroTerminal', HeroTerminal)
    app.component('BenchmarkChart', BenchmarkChart)
  }
} satisfies Theme
