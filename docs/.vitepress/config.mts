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
    sidebar: {
      '/guide/': [
        {
          text: 'Introduction',
          items: [
            { text: 'What is RestPHP?', link: '/guide/what-is-restphp' },
            { text: 'Getting Started', link: '/guide/getting-started' },
            { text: 'CLI Usage', link: '/guide/cli-commands' },
          ]
        },
        {
          text: 'Core Features',
          items: [
            { text: 'Zero Host GC', link: '/guide/zero-host-gc' },
            { text: 'Persistent Worker Model', link: '/guide/persistent-worker' },
            { text: 'Superglobal Mapping', link: '/guide/superglobals' },
          ]
        }
      ],
      '/architecture/': [
        {
          text: 'Architecture Deep-Dive',
          items: [
            { text: 'Architectural Blueprint', link: '/architecture/overview' },
            { text: 'Zend Engine C-FFI', link: '/architecture/zend-ffi' },
            { text: 'Custom SAPI Bridge', link: '/architecture/custom-sapi' },
            { text: 'Async Tokio Engine', link: '/architecture/async-engine' },
          ]
        }
      ],
      '/frameworks/': [
        {
          text: 'Framework Integrations',
          items: [
            { text: 'Laravel & Octane', link: '/frameworks/laravel-octane' },
            { text: 'Traditional PHP & Slim', link: '/frameworks/traditional-php' },
            { text: 'Symfony Runtime', link: '/frameworks/symfony' },
          ]
        }
      ],
      '/benchmarks/': [
        {
          text: 'Performance Benchmarks',
          items: [
            { text: 'RestPHP vs FrankenPHP', link: '/benchmarks/comparison' },
            { text: 'Benchmark Methodology', link: '/benchmarks/methodology' },
          ]
        }
      ]
    },
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
