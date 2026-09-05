import { defineConfig } from 'vitepress'

export default defineConfig({
  base: process.env.NODE_ENV === 'production' ? '/restphp/' : '/',
  title: "RestPHP",
  description: "The Blazing-Fast, Persistent Application Server & Runtime for PHP powered by Rust",
  lastUpdated: true,
  cleanUrls: true,
  head: [
    ['link', { rel: 'icon', href: '/logo.svg' }],
    ['meta', { name: 'theme-color', content: '#f97316' }],
    ['meta', { name: 'og:type', content: 'website' }],
    ['meta', { name: 'og:title', content: 'RestPHP — The Blazing-Fast PHP Runtime powered by Rust' }],
    ['meta', { name: 'og:description', content: 'Zero Host GC. Zero CGO Overhead. Outperforming FrankenPHP, RoadRunner, and Swoole.' }],
  ],
  themeConfig: {
    logo: '/logo.svg',
    siteTitle: 'RestPHP',
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Architecture', link: '/architecture/overview' },
      { text: 'Laravel Octane', link: '/frameworks/laravel-octane' },
      { text: 'Benchmarks', link: '/benchmarks/comparison' },
      { text: 'Roadmap', link: '/roadmap' },
      {
        text: 'v0.1.0',
        items: [
          { text: 'Changelog', link: 'https://github.com/arsyadal/restphp/releases' },
          { text: 'Contributing', link: 'https://github.com/arsyadal/restphp/blob/main/CONTRIBUTING.md' },
        ]
      }
    ],
    sidebar: [
      {
        text: 'Get Started',
        items: [
          { text: 'Welcome to RestPHP', link: '/guide/what-is-restphp' },
          { text: 'Getting Started', link: '/guide/getting-started' },
          { text: 'CLI Reference', link: '/guide/cli-commands' },
        ]
      },
      {
        text: 'Core Runtime',
        items: [
          { text: 'Persistent Worker Architecture', link: '/runtime/persistent-workers' },
          { text: 'Zero Host GC & Determinism', link: '/runtime/zero-host-gc' },
          { text: 'Request Lifecycle & State Reset', link: '/runtime/state-reset' },
        ]
      },
      {
        text: 'HTTP & SAPI Subsystem',
        items: [
          { text: 'Superglobal Mapping', link: '/http/superglobals' },
          { text: 'Output Buffering & Headers', link: '/http/output-buffering' },
        ]
      },
      {
        text: 'Framework Integrations',
        items: [
          { text: 'Laravel & Laravel Octane', link: '/frameworks/laravel-octane' },
          { text: 'Traditional PHP & Slim', link: '/frameworks/traditional-php' },
          { text: 'Symfony Framework', link: '/frameworks/symfony' },
        ]
      },
      {
        text: 'Architecture Deep Dive',
        items: [
          { text: 'Architectural Blueprint', link: '/architecture/overview' },
          { text: 'Zend Engine C-FFI', link: '/architecture/zend-ffi' },
          { text: 'Custom SAPI Bridge (c/sapi.c)', link: '/architecture/custom-sapi' },
          { text: 'Async Tokio Engine', link: '/architecture/async-engine' },
        ]
      },
      {
        text: 'Performance & Benchmarks',
        items: [
          { text: 'RestPHP vs Competitors', link: '/benchmarks/comparison' },
          { text: 'Benchmark Methodology', link: '/benchmarks/methodology' },
        ]
      },
      {
        text: 'Project & Community',
        items: [
          { text: 'Roadmap & Milestones', link: '/roadmap' },
          { text: 'GitHub Repository', link: 'https://github.com/arsyadal/restphp' },
        ]
      }
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/arsyadal/restphp' }
    ],
    footer: {
      message: 'Released under the MIT / Apache-2.0 License. Powered by Rust & Zend Engine.',
      copyright: 'Copyright © 2026 Arsyad Alghital. All rights reserved.'
    },
    search: {
      provider: 'local'
    }
  }
})
