import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import HeroTerminal from './components/HeroTerminal.vue'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('HeroTerminal', HeroTerminal)
  }
} satisfies Theme
