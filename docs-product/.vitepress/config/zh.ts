export const zhConfig = {
  themeConfig: {
    nav: [
      { text: '指南', link: '/zh/guide/' },
      { text: 'API', link: '/zh/api/' },
      { text: '开发', link: '/zh/develop/' },
      { text: '部署', link: '/zh/deploy/' },
      { text: 'FAQ', link: '/zh/faq/' },
      {
        text: '版本',
        items: [
          { text: '更新日志', link: '/zh/releases/changelog' },
          { text: '路线图', link: '/zh/releases/roadmap' }
        ]
      }
    ],

    sidebar: {
      '/zh/guide/': [
        {
          text: '入门',
          items: [
            { text: '快速开始', link: '/zh/guide/' },
            { text: '安装指南', link: '/zh/guide/installation' },
            { text: '版本选择', link: '/zh/guide/editions' }
          ]
        },
        {
          text: '功能详解',
          items: [
            { text: 'Lite 版', link: '/zh/guide/features/lite' },
            { text: 'Standard 版', link: '/zh/guide/features/standard' },
            { text: 'Pro 版', link: '/zh/guide/features/pro' },
            { text: '终端功能', link: '/zh/guide/features/terminal' },
            { text: 'SFTP 传输', link: '/zh/guide/features/sftp' },
            { text: '团队协作', link: '/zh/guide/features/team' }
          ]
        },
        {
          text: '进阶',
          items: [
            { text: '导入配置', link: '/zh/guide/import-config' },
            { text: '密钥管理', link: '/zh/guide/key-management' },
            { text: '同步设置', link: '/zh/guide/sync' },
            { text: '快捷键', link: '/zh/guide/shortcuts' }
          ]
        }
      ],

      '/zh/api/': [
        {
          text: 'API 参考',
          items: [
            { text: '概览', link: '/zh/api/' },
            { text: 'FFI 接口', link: '/zh/api/ffi' }
          ]
        },
        {
          text: 'Core 库',
          items: [
            { text: 'SSH 模块', link: '/zh/api/core/ssh' },
            { text: '数据库', link: '/zh/api/core/db' },
            { text: '加密', link: '/zh/api/core/crypto' },
            { text: 'SFTP', link: '/zh/api/core/sftp' },
            { text: '终端', link: '/zh/api/core/terminal' },
            { text: '布局', link: '/zh/api/core/layout' }
          ]
        },
        {
          text: 'Pro 功能',
          items: [
            { text: '团队管理', link: '/zh/api/pro/team' },
            { text: 'RBAC', link: '/zh/api/pro/rbac' },
            { text: '审计', link: '/zh/api/pro/audit' }
          ]
        }
      ],

      '/zh/develop/': [
        {
          text: '开发指南',
          items: [
            { text: '入门', link: '/zh/develop/' },
            { text: '架构说明', link: '/zh/develop/architecture' },
            { text: '贡献指南', link: '/zh/develop/contributing' },
            { text: '构建指南', link: '/zh/develop/building' },
            { text: '测试指南', link: '/zh/develop/testing' },
            { text: '代码规范', link: '/zh/develop/coding-standards' }
          ]
        },
        {
          text: '高级主题',
          items: [
            { text: '添加平台', link: '/zh/develop/add-platform' },
            { text: '插件开发', link: '/zh/develop/plugins' },
            { text: '性能优化', link: '/zh/develop/performance' }
          ]
        }
      ],

      '/zh/deploy/': [
        {
          text: '部署指南',
          items: [
            { text: '概览', link: '/zh/deploy/' },
            { text: '企业部署', link: '/zh/deploy/enterprise' },
            { text: '安全配置', link: '/zh/deploy/security' },
            { text: '扩展指南', link: '/zh/deploy/scaling' }
          ]
        },
        {
          text: '配置参考',
          items: [
            { text: '服务端配置', link: '/zh/deploy/server-config' },
            { text: '客户端配置', link: '/zh/deploy/client-config' },
            { text: '环境变量', link: '/zh/deploy/environment' }
          ]
        }
      ],

      '/zh/faq/': [
        {
          text: '常见问题',
          items: [
            { text: '全部问题', link: '/zh/faq/' },
            { text: '一般问题', link: '/zh/faq/general' },
            { text: 'Lite 版', link: '/zh/faq/lite' },
            { text: 'Standard 版', link: '/zh/faq/standard' },
            { text: 'Pro 版', link: '/zh/faq/pro' }
          ]
        }
      ],

      '/zh/releases/': [
        {
          text: '版本信息',
          items: [
            { text: '版本历史', link: '/zh/releases/' },
            { text: '更新日志', link: '/zh/releases/changelog' },
            { text: '迁移指南', link: '/zh/releases/migration' },
            { text: '路线图', link: '/zh/releases/roadmap' }
          ]
        }
      ],

      '/zh/troubleshooting/': [
        {
          text: '故障排查',
          items: [
            { text: '概览', link: '/zh/troubleshooting/' },
            { text: '错误代码', link: '/zh/troubleshooting/error-codes' },
            { text: '连接问题', link: '/zh/troubleshooting/connection' },
            { text: '认证问题', link: '/zh/troubleshooting/authentication' },
            { text: '性能问题', link: '/zh/troubleshooting/performance' },
            { text: '日志收集', link: '/zh/troubleshooting/log-collection' }
          ]
        }
      ]
    },

    editLink: {
      pattern: 'https://github.com/anixops/easyssh/edit/main/docs-product/:path',
      text: '在 GitHub 上编辑此页'
    },

    docFooter: {
      prev: '上一页',
      next: '下一页'
    },

    outline: {
      label: '页面导航'
    },

    lastUpdated: {
      text: '最后更新于'
    }
  }
}
