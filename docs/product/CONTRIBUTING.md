# 文档项目

EasySSH 产品文档 - VitePress 构建

## 快速开始

```bash
# 安装依赖
pnpm install

# 开发模式
pnpm docs:dev

# 构建
pnpm docs:build

# 预览
pnpm docs:preview
```

## 项目结构

```
docs-product/
├── .vitepress/          # VitePress 配置
│   ├── config.ts
│   └── theme/
├── zh/                  # 中文文档
├── en/                  # 英文文档
├── ja/                  # 日文文档
├── public/              # 静态资源
│   └── images/
├── package.json
└── README.md
```

## 编写文档

### 创建新页面

1. 在对应语言目录创建 `.md` 文件
2. 在 `config.ts` 中添加导航配置
3. 使用 frontmatter 设置页面信息：

```yaml
---
title: 页面标题
description: 页面描述
---
```

### 组件使用

```md
::: tip 提示
这是提示内容
:::

::: warning 警告
这是警告内容
:::

::: danger 危险
这是危险警告
:::
```

### 代码块

```md
```rust
fn main() {
    println!("Hello, EasySSH!");
}
```
```

## 多语言

- 中文: `zh/` 目录
- 英文: `en/` 目录
- 日文: `ja/` 目录

## 部署

文档自动部署到 GitHub Pages：
- 主站: https://docs.easyssh.dev
- 预览: https://docs-staging.easyssh.dev

## 贡献

1. Fork 仓库
2. 创建分支: `git checkout -b docs/update`
3. 提交更改: `git commit -m 'docs: update guide'`
4. 推送: `git push origin docs/update`
5. 创建 PR

## 联系

docs@easyssh.dev
