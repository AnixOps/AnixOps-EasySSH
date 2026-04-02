export const jaConfig = {
  themeConfig: {
    nav: [
      { text: 'ガイド', link: '/ja/guide/' },
      { text: 'API', link: '/ja/api/' },
      { text: '開発', link: '/ja/develop/' },
      { text: 'デプロイ', link: '/ja/deploy/' },
      { text: 'FAQ', link: '/ja/faq/' },
      {
        text: 'リリース',
        items: [
          { text: '変更履歴', link: '/ja/releases/changelog' },
          { text: 'ロードマップ', link: '/ja/releases/roadmap' }
        ]
      }
    ],

    sidebar: {
      '/ja/guide/': [
        {
          text: 'はじめに',
          items: [
            { text: 'クイックスタート', link: '/ja/guide/' },
            { text: 'インストール', link: '/ja/guide/installation' },
            { text: 'エディションガイド', link: '/ja/guide/editions' }
          ]
        },
        {
          text: '機能詳細',
          items: [
            { text: 'Lite エディション', link: '/ja/guide/features/lite' },
            { text: 'Standard エディション', link: '/ja/guide/features/standard' },
            { text: 'Pro エディション', link: '/ja/guide/features/pro' },
            { text: 'ターミナル', link: '/ja/guide/features/terminal' },
            { text: 'SFTP 転送', link: '/ja/guide/features/sftp' },
            { text: 'チーム協業', link: '/ja/guide/features/team' }
          ]
        },
        {
          text: '高度な機能',
          items: [
            { text: '設定のインポート', link: '/ja/guide/import-config' },
            { text: 'キー管理', link: '/ja/guide/key-management' },
            { text: '同期設定', link: '/ja/guide/sync' },
            { text: 'キーボードショートカット', link: '/ja/guide/shortcuts' }
          ]
        }
      ],

      '/ja/api/': [
        {
          text: 'API リファレンス',
          items: [
            { text: '概要', link: '/ja/api/' },
            { text: 'FFI インターフェース', link: '/ja/api/ffi' }
          ]
        },
        {
          text: 'Core ライブラリ',
          items: [
            { text: 'SSH モジュール', link: '/ja/api/core/ssh' },
            { text: 'データベース', link: '/ja/api/core/db' },
            { text: '暗号化', link: '/ja/api/core/crypto' },
            { text: 'SFTP', link: '/ja/api/core/sftp' },
            { text: 'ターミナル', link: '/ja/api/core/terminal' },
            { text: 'レイアウト', link: '/ja/api/core/layout' }
          ]
        },
        {
          text: 'Pro 機能',
          items: [
            { text: 'チーム管理', link: '/ja/api/pro/team' },
            { text: 'RBAC', link: '/ja/api/pro/rbac' },
            { text: '監査', link: '/ja/api/pro/audit' }
          ]
        }
      ],

      '/ja/develop/': [
        {
          text: '開発ガイド',
          items: [
            { text: 'はじめに', link: '/ja/develop/' },
            { text: 'アーキテクチャ', link: '/ja/develop/architecture' },
            { text: '貢献ガイド', link: '/ja/develop/contributing' },
            { text: 'ビルドガイド', link: '/ja/develop/building' },
            { text: 'テストガイド', link: '/ja/develop/testing' },
            { text: 'コーディング規約', link: '/ja/develop/coding-standards' }
          ]
        },
        {
          text: '高度なトピック',
          items: [
            { text: 'プラットフォーム追加', link: '/ja/develop/add-platform' },
            { text: 'プラグイン開発', link: '/ja/develop/plugins' },
            { text: 'パフォーマンス', link: '/ja/develop/performance' }
          ]
        }
      ],

      '/ja/deploy/': [
        {
          text: 'デプロイガイド',
          items: [
            { text: '概要', link: '/ja/deploy/' },
            { text: 'エンタープライズ', link: '/ja/deploy/enterprise' },
            { text: 'セキュリティ', link: '/ja/deploy/security' },
            { text: 'スケーリング', link: '/ja/deploy/scaling' }
          ]
        },
        {
          text: '設定リファレンス',
          items: [
            { text: 'サーバー設定', link: '/ja/deploy/server-config' },
            { text: 'クライアント設定', link: '/ja/deploy/client-config' },
            { text: '環境変数', link: '/ja/deploy/environment' }
          ]
        }
      ],

      '/ja/faq/': [
        {
          text: 'よくある質問',
          items: [
            { text: 'すべての質問', link: '/ja/faq/' },
            { text: '一般的', link: '/ja/faq/general' },
            { text: 'Lite エディション', link: '/ja/faq/lite' },
            { text: 'Standard エディション', link: '/ja/faq/standard' },
            { text: 'Pro エディション', link: '/ja/faq/pro' }
          ]
        }
      ],

      '/ja/releases/': [
        {
          text: 'リリース情報',
          items: [
            { text: 'バージョン履歴', link: '/ja/releases/' },
            { text: '変更履歴', link: '/ja/releases/changelog' },
            { text: '移行ガイド', link: '/ja/releases/migration' },
            { text: 'ロードマップ', link: '/ja/releases/roadmap' }
          ]
        }
      ],

      '/ja/troubleshooting/': [
        {
          text: 'トラブルシューティング',
          items: [
            { text: '概要', link: '/ja/troubleshooting/' },
            { text: 'エラーコード', link: '/ja/troubleshooting/error-codes' },
            { text: '接続問題', link: '/ja/troubleshooting/connection' },
            { text: '認証問題', link: '/ja/troubleshooting/authentication' },
            { text: 'パフォーマンス', link: '/ja/troubleshooting/performance' },
            { text: 'ログ収集', link: '/ja/troubleshooting/log-collection' }
          ]
        }
      ]
    },

    editLink: {
      pattern: 'https://github.com/anixops/easyssh/edit/main/docs-product/:path',
      text: 'GitHub でこのページを編集'
    },

    docFooter: {
      prev: '前へ',
      next: '次へ'
    },

    outline: {
      label: 'ページ目次'
    },

    lastUpdated: {
      text: '最終更新'
    }
  }
}
