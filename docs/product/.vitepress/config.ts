import { defineConfig } from 'vitepress'
import { zhConfig } from './config/zh'
import { enConfig } from './config/en'
import { jaConfig } from './config/ja'

export default defineConfig({
  title: 'EasySSH Documentation',
  description: 'Secure, modern SSH client for developers and teams',

  head: [
    ['link', { rel: 'icon', href: '/logo.svg' }],
    ['meta', { name: 'theme-color', content: '#3c3c3c' }],
    ['meta', { name: 'og:type', content: 'website' }],
    ['meta', { name: 'og:locale', content: 'zh-CN' }],
    ['meta', { name: 'og:site_name', content: 'EasySSH Documentation' }],
    ['script', { src: '/analytics.js', defer: '' }]
  ],

  cleanUrls: true,
  lastUpdated: true,

  locales: {
    root: {
      label: '简体中文',
      lang: 'zh',
      link: '/zh/',
      ...zhConfig
    },
    en: {
      label: 'English',
      lang: 'en',
      link: '/en/',
      ...enConfig
    },
    ja: {
      label: '日本語',
      lang: 'ja',
      link: '/ja/',
      ...jaConfig
    }
  },

  themeConfig: {
    logo: '/logo.svg',

    socialLinks: [
      { icon: 'github', link: 'https://github.com/anixops/easyssh' },
      { icon: 'twitter', link: 'https://twitter.com/easyssh' },
      { icon: 'discord', link: 'https://discord.gg/easyssh' }
    ],

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026 EasySSH Team'
    },

    search: {
      provider: 'algolia',
      options: {
        appId: 'YOUR_APP_ID',
        apiKey: 'YOUR_API_KEY',
        indexName: 'easyssh'
      }
    }
  },

  markdown: {
    lineNumbers: true,
    config: (md) => {
      md.use(require('markdown-it-container'), 'tabs')
      md.use(require('markdown-it-container'), 'tip')
      md.use(require('markdown-it-container'), 'warning')
      md.use(require('markdown-it-container'), 'danger')
    }
  }
})
