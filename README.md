# 青拾客户端模板 (qingshi-khd-muban)

基于 Tauri 2 + Vue 3 + TypeScript 的青拾客户端开发模板。

## 技术栈

- **桌面框架**: Tauri 2
- **前端框架**: Vue 3 (Composition API + `<script setup>`)
- **UI 组件库**: PrimeVue 4
- **状态管理**: Pinia
- **路由**: Vue Router 4
- **构建工具**: Vite 6
- **语言**: TypeScript + Rust

## 使用模板创建新项目

1. 在 GitHub 上点击 **Use this template** 创建新仓库
2. 克隆新仓库到本地
3. 修改项目名称：
   - `package.json` 中的 `name`
   - `src-tauri/Cargo.toml` 中的 `name` 和 `[lib] name`
   - `src-tauri/src/main.rs` 中的 lib 引用
   - `src-tauri/tauri.conf.json` 中的 `productName` 和 `identifier`
4. 复制 `.env.example` 为 `.env` 并配置 API 地址
5. 安装依赖并启动开发

## 开发

```bash
# 安装前端依赖
npm install

# 启动开发模式（Tauri + Vite）
npm run tauri dev

# 构建生产版本
npm run tauri build
```

## 项目结构

```
├── src/                  # Vue 前端源码
├── src-tauri/            # Tauri/Rust 后端源码
│   ├── src/              # Rust 源码
│   ├── icons/            # 应用图标
│   ├── Cargo.toml        # Rust 依赖配置
│   └── tauri.conf.json   # Tauri 配置
├── public/               # 静态资源
├── .env.example          # 环境变量示例
├── package.json          # 前端依赖配置
├── vite.config.ts        # Vite 配置
└── tsconfig.json         # TypeScript 配置
```

## 推荐 IDE

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
