import { defineConfig } from 'vitepress'

export default defineConfig({
  title: "YYKV Documentation",
  description: "Bare Metal Storage Kernel",
  locales: {
    root: {
      label: 'English',
      lang: 'en-US',
      link: '/en-us/',
      themeConfig: {
        nav: [
          { text: 'Guide', link: '/en-us/guide/' },
          { text: 'Reference', link: '/en-us/reference/' }
        ],
        sidebar: [
          {
            text: 'Guide',
            items: [
              { text: 'Introduction', link: '/en-us/guide/' },
              { text: 'Architecture', link: '/en-us/guide/architecture' }
            ]
          },
          {
            text: 'Protocols',
            collapsed: false,
            items: [
              { text: 'Overview', link: '/en-us/protocol/' },
              { text: 'We-Trust (Native)', link: '/en-us/protocol/we-trust' },
              { text: 'PostgreSQL', link: '/en-us/protocol/postgres' },
              { text: 'MySQL', link: '/en-us/protocol/mysql' },
              { text: 'Redis', link: '/en-us/protocol/redis' },
              { text: 'S3 Object Storage', link: '/en-us/protocol/s3' },
              { text: 'SQLite', link: '/en-us/protocol/sqlite' },
              { text: 'SQL Server', link: '/en-us/protocol/sqlserver' },
              { text: 'Apache Arrow', link: '/en-us/protocol/arrow' },
              { text: 'Vector Search', link: '/en-us/protocol/vector' },
              { text: 'OpenTelemetry', link: '/en-us/protocol/otel' },
              { text: 'Cloudflare D1', link: '/en-us/protocol/d1' }
            ]
          },
          {
            text: 'Reference',
            items: [
              { text: 'Overview', link: '/en-us/reference/' }
            ]
          }
        ]
      }
    },
    zh: {
      label: '简体中文',
      lang: 'zh-Hans',
      link: '/zh-hans/',
      themeConfig: {
        nav: [
          { text: '指南', link: '/zh-hans/guide/' },
          { text: '参考', link: '/zh-hans/reference/' }
        ],
        sidebar: [
          {
            text: '指南',
            items: [
              { text: '介绍', link: '/zh-hans/guide/' },
              { text: '架构设计', link: '/zh-hans/guide/architecture' }
            ]
          },
          {
            text: '协议',
            collapsed: false,
            items: [
              { text: '概览', link: '/zh-hans/protocol/' },
              { text: 'We-Trust (原生)', link: '/zh-hans/protocol/we-trust' },
              { text: 'PostgreSQL', link: '/zh-hans/protocol/postgres' },
              { text: 'MySQL', link: '/zh-hans/protocol/mysql' },
              { text: 'Redis', link: '/zh-hans/protocol/redis' },
              { text: 'S3 对象存储', link: '/zh-hans/protocol/s3' },
              { text: 'SQLite', link: '/zh-hans/protocol/sqlite' },
              { text: 'SQL Server', link: '/zh-hans/protocol/sqlserver' },
              { text: 'Apache Arrow', link: '/zh-hans/protocol/arrow' },
              { text: 'Vector 向量', link: '/zh-hans/protocol/vector' },
              { text: 'OpenTelemetry', link: '/zh-hans/protocol/otel' },
              { text: 'Cloudflare D1', link: '/zh-hans/protocol/d1' }
            ]
          },
          {
            text: '参考',
            items: [
              { text: '概览', link: '/zh-hans/reference/' }
            ]
          }
        ]
      }
    }
  },
  themeConfig: {
    socialLinks: [
      { icon: 'github', link: 'https://github.com/yydb/yydb' }
    ]
  }
})
