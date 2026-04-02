export const enConfig = {
  themeConfig: {
    nav: [
      { text: 'Guide', link: '/en/guide/' },
      { text: 'API', link: '/en/api/' },
      { text: 'Develop', link: '/en/develop/' },
      { text: 'Deploy', link: '/en/deploy/' },
      { text: 'FAQ', link: '/en/faq/' },
      {
        text: 'Releases',
        items: [
          { text: 'Changelog', link: '/en/releases/changelog' },
          { text: 'Roadmap', link: '/en/releases/roadmap' }
        ]
      }
    ],

    sidebar: {
      '/en/guide/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Quick Start', link: '/en/guide/' },
            { text: 'Installation', link: '/en/guide/installation' },
            { text: 'Edition Guide', link: '/en/guide/editions' }
          ]
        },
        {
          text: 'Features',
          items: [
            { text: 'Lite Edition', link: '/en/guide/features/lite' },
            { text: 'Standard Edition', link: '/en/guide/features/standard' },
            { text: 'Pro Edition', link: '/en/guide/features/pro' },
            { text: 'Terminal', link: '/en/guide/features/terminal' },
            { text: 'SFTP Transfer', link: '/en/guide/features/sftp' },
            { text: 'Team Collaboration', link: '/en/guide/features/team' }
          ]
        },
        {
          text: 'Advanced',
          items: [
            { text: 'Import Config', link: '/en/guide/import-config' },
            { text: 'Key Management', link: '/en/guide/key-management' },
            { text: 'Sync Settings', link: '/en/guide/sync' },
            { text: 'Keyboard Shortcuts', link: '/en/guide/shortcuts' }
          ]
        }
      ],

      '/en/api/': [
        {
          text: 'API Reference',
          items: [
            { text: 'Overview', link: '/en/api/' },
            { text: 'FFI Interface', link: '/en/api/ffi' }
          ]
        },
        {
          text: 'Core Library',
          items: [
            { text: 'SSH Module', link: '/en/api/core/ssh' },
            { text: 'Database', link: '/en/api/core/db' },
            { text: 'Crypto', link: '/en/api/core/crypto' },
            { text: 'SFTP', link: '/en/api/core/sftp' },
            { text: 'Terminal', link: '/en/api/core/terminal' },
            { text: 'Layout', link: '/en/api/core/layout' }
          ]
        },
        {
          text: 'Pro Features',
          items: [
            { text: 'Team Management', link: '/en/api/pro/team' },
            { text: 'RBAC', link: '/en/api/pro/rbac' },
            { text: 'Audit', link: '/en/api/pro/audit' }
          ]
        }
      ],

      '/en/develop/': [
        {
          text: 'Developer Guide',
          items: [
            { text: 'Getting Started', link: '/en/develop/' },
            { text: 'Architecture', link: '/en/develop/architecture' },
            { text: 'Contributing', link: '/en/develop/contributing' },
            { text: 'Building', link: '/en/develop/building' },
            { text: 'Testing', link: '/en/develop/testing' },
            { text: 'Coding Standards', link: '/en/develop/coding-standards' }
          ]
        },
        {
          text: 'Advanced Topics',
          items: [
            { text: 'Add Platform', link: '/en/develop/add-platform' },
            { text: 'Plugin Development', link: '/en/develop/plugins' },
            { text: 'Performance', link: '/en/develop/performance' }
          ]
        }
      ],

      '/en/deploy/': [
        {
          text: 'Deployment',
          items: [
            { text: 'Overview', link: '/en/deploy/' },
            { text: 'Enterprise', link: '/en/deploy/enterprise' },
            { text: 'Security', link: '/en/deploy/security' },
            { text: 'Scaling', link: '/en/deploy/scaling' }
          ]
        },
        {
          text: 'Configuration',
          items: [
            { text: 'Server Config', link: '/en/deploy/server-config' },
            { text: 'Client Config', link: '/en/deploy/client-config' },
            { text: 'Environment', link: '/en/deploy/environment' }
          ]
        }
      ],

      '/en/faq/': [
        {
          text: 'FAQ',
          items: [
            { text: 'All Questions', link: '/en/faq/' },
            { text: 'General', link: '/en/faq/general' },
            { text: 'Lite Edition', link: '/en/faq/lite' },
            { text: 'Standard Edition', link: '/en/faq/standard' },
            { text: 'Pro Edition', link: '/en/faq/pro' }
          ]
        }
      ],

      '/en/releases/': [
        {
          text: 'Release Info',
          items: [
            { text: 'Version History', link: '/en/releases/' },
            { text: 'Changelog', link: '/en/releases/changelog' },
            { text: 'Migration Guide', link: '/en/releases/migration' },
            { text: 'Roadmap', link: '/en/releases/roadmap' }
          ]
        }
      ],

      '/en/troubleshooting/': [
        {
          text: 'Troubleshooting',
          items: [
            { text: 'Overview', link: '/en/troubleshooting/' },
            { text: 'Error Codes', link: '/en/troubleshooting/error-codes' },
            { text: 'Connection Issues', link: '/en/troubleshooting/connection' },
            { text: 'Authentication', link: '/en/troubleshooting/authentication' },
            { text: 'Performance', link: '/en/troubleshooting/performance' },
            { text: 'Log Collection', link: '/en/troubleshooting/log-collection' }
          ]
        }
      ]
    },

    editLink: {
      pattern: 'https://github.com/anixops/easyssh/edit/main/docs-product/:path',
      text: 'Edit this page on GitHub'
    },

    docFooter: {
      prev: 'Previous',
      next: 'Next'
    },

    outline: {
      label: 'On this page'
    },

    lastUpdated: {
      text: 'Last updated'
    }
  }
}
