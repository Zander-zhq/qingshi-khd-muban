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

## 窗口架构

采用**单窗口 + 隐身换装**架构。整个应用只有一个 Tauri 窗口（label: `main`），登录页和主页通过 Vue Router 在同一个窗口内切换，窗口尺寸由 Rust 端原子操作完成（hide → resize → center → show），用户看不到尺寸变化过程。

### 布局切换流程

| 场景 | 流程 |
|------|------|
| 登录 → 主页 | Rust: hide → 1440x900 → center → JS: router.push → Rust: show |
| 主页 → 登录 | Rust: hide → 420x640 → center → JS: router.push → Rust: show |

### Rust 命令

| 命令 | 说明 |
|------|------|
| `prepare_window` | 隐藏窗口 + 设置尺寸/最小尺寸/可缩放 + 居中 |
| `reveal_window` | 显示窗口 + 聚焦 |
| `exit_app` | 退出应用 |
| `compute_sign` | 请求签名计算 |
| `get_device_id` | 获取设备唯一标识 |
| `get_app_credentials` | 获取应用凭证 |

### 前端工具函数 (`src/utils/window.ts`)

| 函数 | 说明 |
|------|------|
| `switchToMainLayout(router)` | 切换到主布局（hide → resize → push → show） |
| `switchToLoginLayout(router)` | 切换到登录布局（hide → resize → push → show） |
| `showWindow()` | 显示窗口 |
| `exitApp()` | 退出应用 |

## 项目结构

```
├── src/                  # Vue 前端源码
│   ├── api/              # API 请求
│   ├── components/       # 公共组件（TitleBar 等）
│   ├── layouts/          # 布局组件（MainLayout）
│   ├── router/           # 路由配置
│   ├── stores/           # Pinia 状态管理
│   ├── utils/            # 工具函数（window、logger 等）
│   └── views/            # 页面视图
├── src-tauri/            # Tauri/Rust 后端源码
│   ├── src/              # Rust 源码
│   ├── icons/            # 应用图标
│   ├── capabilities/     # 权限配置
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
