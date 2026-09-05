import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import HeroTerminal from './components/HeroTerminal.vue'
import BenchmarkChart from './components/BenchmarkChart.vue'
import ScenarioCards from './components/ScenarioCards.vue'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('HeroTerminal', HeroTerminal)
    app.component('BenchmarkChart', BenchmarkChart)
    app.component('ScenarioCards', ScenarioCards)
  }
} satisfies Theme
