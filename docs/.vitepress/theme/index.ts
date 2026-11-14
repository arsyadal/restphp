import DefaultTheme from 'vitepress/theme'
import { h } from 'vue'
import type { Theme } from 'vitepress'
import HeroTerminal from './components/HeroTerminal.vue'
import BenchmarkChart from './components/BenchmarkChart.vue'
import ScenarioCards from './components/ScenarioCards.vue'
import GitHubStars from './components/GitHubStars.vue'

export default {
  extends: DefaultTheme,
  Layout() {
    return h(DefaultTheme.Layout, null, {
      'nav-bar-content-after': () => h(GitHubStars)
    })
  },
  enhanceApp({ app }) {
    app.component('HeroTerminal', HeroTerminal)
    app.component('BenchmarkChart', BenchmarkChart)
    app.component('ScenarioCards', ScenarioCards)
  }
} satisfies Theme
