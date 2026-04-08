# 青拾客户端模板 (qingshi-khd-muban)

基于 Tauri 2 + Vue 3 + TypeScript 的桌面客户端基础框架。

所有产品（视频下载、AI视频、剪辑等）共用此模板，每个产品只需修改配置和添加自己的业务页面。

## 技术栈

- **桌面框架**: Tauri 2（Rust 后端）
- **前端框架**: Vue 3（Composition API + `<script setup>`）
- **UI 组件库**: PrimeVue 4 / PrimeIcons
- **状态管理**: Pinia
- **路由**: Vue Router 4
- **构建工具**: Vite 6
- **语言**: TypeScript + Rust

---

## 项目结构

```
├── src/                          # 前端源码
│   ├── app/                      # [产品专属] 业务代码目录
│   │   ├── pages/                #   业务页面组件
│   │   └── routes.ts             #   业务路由定义
│   ├── api/                      # [基础框架] 通用 API
│   │   ├── auth.ts               #   登录/注册/找回密码/卡密充值
│   │   ├── brand.ts              #   品牌配置同步
│   │   ├── pay.ts                #   在线支付（登录版 + 免登录版）
│   │   └── version.ts            #   版本管理（检查更新/发布/上传）
│   ├── composables/              # [基础框架] 组合式函数
│   │   ├── useGuestPay.ts        #   免登录在线支付逻辑
│   │   └── useVersionUpdate.ts   #   版本更新检查与安装
│   ├── components/               # [基础框架] 公共组件
│   │   └── TitleBar.vue          #   自定义窗口标题栏
│   ├── stores/                   # [基础框架] Pinia 状态管理
│   │   └── user.ts               #   用户状态（登录/token/信息）
│   ├── utils/                    # [基础框架] 工具函数
│   │   ├── request.ts            #   Axios 封装 + HMAC 签名
│   │   ├── sign.ts               #   签名计算（调 Rust 端）
│   │   ├── config.ts             #   应用凭证获取
│   │   ├── window.ts             #   窗口切换（登录↔主页）
│   │   ├── storage.ts            #   localStorage 封装
│   │   ├── dialog.ts             #   弹窗工具
│   │   └── logger.ts             #   日志工具
│   ├── templates/                # [基础框架] UI 模板（三套皮肤）
│   │   ├── green/                #   绿色主题
│   │   ├── orange/               #   橙色主题
│   │   └── dark/                 #   暗黑主题
│   ├── dev/                      # [基础框架] 开发工具（仅 DEV 模式可见）
│   │   ├── BrandManager.vue      #   品牌配置管理
│   │   └── VersionManager.vue    #   版本打包发布
│   ├── router/                   # [基础框架] 路由
│   │   └── index.ts              #   核心路由 + 导入 app/routes.ts
│   ├── brand.ts                  # [基础框架] 品牌配置系统
│   └── vite-env.d.ts             #   Vite 环境变量类型
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── app_config.rs         # [产品专属] APP_ID / APP_KEY 配置
│   │   ├── lib.rs                # [基础框架] 核心命令（签名/窗口/构建等）
│   │   └── main.rs               #   入口
│   ├── icons/                    #   应用图标
│   ├── Cargo.toml                #   Rust 依赖
│   └── tauri.conf.json           #   Tauri 窗口/打包配置
├── public/                       # 静态资源
│   ├── app-icon.png              #   默认应用图标
│   └── brand.json                #   默认品牌配置（会被服务端覆盖）
├── .env.example                  # 环境变量示例
├── package.json                  # 前端依赖
└── vite.config.ts                # Vite 构建配置
```

### 关键分区规则

| 标记 | 含义 | 产品是否修改 |
|------|------|-------------|
| `[基础框架]` | 所有产品共用的代码 | 不修改，通过 upstream 同步更新 |
| `[产品专属]` | 每个产品自己的代码 | 自由修改，不会与模板冲突 |

**产品开发只需要关注两个地方：**
1. `src-tauri/src/app_config.rs` — 填入产品的 APP_ID 和 APP_KEY
2. `src/app/` — 编写业务页面和路由

---

## 基础功能清单

模板已内置以下功能，所有产品开箱即用：

| 功能 | 说明 |
|------|------|
| 用户认证 | 登录 / 注册 / 找回密码 / 解绑设备 |
| 充值系统 | 卡密充值 + 在线支付（微信/支付宝，支持登录和免登录两种模式） |
| 品牌系统 | 三套 UI 模板（绿/橙/暗黑），品牌名/Logo/联系方式等从服务端同步 |
| 版本管理 | 检查更新 / 下载安装 / 打包发布（DEV 模式） |
| 窗口管理 | 自定义标题栏 / 窗口尺寸切换 / 托盘菜单 / 开机自启 |
| 安全签名 | HMAC-SHA256 请求签名（Rust 端计算） |
| 设备指纹 | 基于硬件信息生成设备唯一 ID |

---

## 开发命令

```bash
# 安装前端依赖
npm install

# 启动开发模式（Tauri + Vite HMR）
npm run tauri dev

# TypeScript 类型检查
npx vue-tsc --noEmit

# 构建生产版本（输出 .exe 安装包）
npm run tauri build
```

---

## 创建新产品（完整步骤）

### 1. 从模板创建仓库

在 GitHub 上打开模板仓库，点击 **Use this template** → **Create a new repository**，输入新产品的仓库名（如 `client_aivideo`）。

```bash
# 克隆新仓库
git clone https://github.com/你的用户名/client_aivideo.git
cd client_aivideo
```

### 2. 关联上游模板（用于以后同步基础框架更新）

```bash
git remote add upstream https://github.com/Zander-zhq/qingshi-khd-muban.git
```

### 3. 修改产品配置

#### 3.1 填入产品凭证

编辑 `src-tauri/src/app_config.rs`：

```rust
pub const APP_ID: &str = "你的产品ID";
pub const APP_KEY: &str = "你的产品密钥";
```

#### 3.2 修改应用标识

编辑 `src-tauri/tauri.conf.json`：

```json
{
  "productName": "你的产品名",
  "identifier": "com.yourcompany.yourapp"
}
```

编辑 `package.json`：

```json
{
  "name": "你的产品包名"
}
```

编辑 `src-tauri/Cargo.toml`：

```toml
[package]
name = "你的产品包名"

[lib]
name = "你的产品包名_lib"
```

编辑 `src-tauri/src/main.rs`（与 Cargo.toml 的 lib name 对应）：

```rust
fn main() {
    你的产品包名_lib::run()
}
```

#### 3.3 配置 API 地址

```bash
cp .env.example .env
# 编辑 .env，设置 VITE_API_BASE_URL
```

### 4. 开发业务页面

在 `src/app/pages/` 下创建业务页面，在 `src/app/routes.ts` 中注册路由：

```typescript
// src/app/routes.ts
import type { RouteRecordRaw } from 'vue-router'
import { getBrand } from '../brand'

const brand = getBrand()

const appRoutes: RouteRecordRaw[] = [
  {
    path: 'editor',
    name: 'Editor',
    component: () => import('./pages/EditorView.vue'),
    meta: { title: `编辑器 - ${brand.brand_name}`, requiresAuth: true },
  },
  // 添加更多业务路由...
]

export default appRoutes
```

### 5. 安装依赖并启动

```bash
npm install
npm run tauri dev
```

---

## 同步基础框架更新

当模板仓库有更新（如修复了充值 bug、增加了新的基础功能）：

```bash
# 拉取模板最新代码
git fetch upstream

# 合并到当前分支
git merge upstream/master

# 如有冲突，解决后提交
# 通常只有 app_config.rs 可能冲突（因为你改了 APP_ID/KEY）
git add .
git commit -m "merge: sync base framework updates"
```

---

## 窗口架构

采用**单窗口 + 隐身换装**架构。整个应用只有一个 Tauri 窗口（label: `main`），登录页和主页通过 Vue Router 在同一个窗口内切换，窗口尺寸由 Rust 端原子操作完成（hide → resize → center → show）。

| 场景 | 流程 |
|------|------|
| 登录 → 主页 | Rust: hide → 1440x900 → center → JS: router.push → Rust: show |
| 主页 → 登录 | Rust: hide → 420x640 → center → JS: router.push → Rust: show |

---

## Rust 命令一览

| 命令 | 说明 |
|------|------|
| `get_app_credentials` | 获取 APP_ID 和 APP_KEY |
| `compute_sign` | HMAC-SHA256 请求签名 |
| `get_device_id` | 获取设备唯一标识 |
| `prepare_window` | 隐藏窗口 + 设置尺寸 + 居中 |
| `reveal_window` | 显示窗口 + 聚焦 |
| `exit_app` | 退出应用 |
| `sync_tray_checks` | 同步托盘菜单选中状态 |
| `read_brand_config` | 读取加密品牌配置 |
| `save_brand_config` | 保存加密品牌配置 |
| `decrypt_brand_config` | 解密品牌配置 |
| `update_tray` | 更新托盘图标和提示 |
| `is_build_running` | 查询构建是否在运行 |
| `start_brand_build` | 启动品牌打包构建 |
| `start_version_build` | 启动版本打包构建（含版本号注入） |
| `read_file_base64` | 读取文件并 Base64 编码（用于上传） |
| `download_file_to_dir` | 下载文件到指定目录（用于版本更新） |
| `get_download_dir` | 获取下载目录路径 |
| `run_installer_and_exit` | 运行安装包并退出当前应用 |

---

## 前端工具函数

### 窗口操作 (`src/utils/window.ts`)

| 函数 | 说明 |
|------|------|
| `switchToMainLayout(router)` | 切换到主布局（1440x900） |
| `switchToLoginLayout(router)` | 切换到登录布局（420x640） |
| `showWindow()` | 显示窗口 |
| `exitApp()` | 退出应用 |

### API 请求 (`src/utils/request.ts`)

| 函数 | 说明 |
|------|------|
| `post(url, data)` | 带 HMAC 签名的 POST 请求（用于 `/client/*` 接口） |

### 品牌配置 (`src/brand.ts`)

| 函数 | 说明 |
|------|------|
| `getBrand()` | 获取当前品牌配置 |
| `setBrand(config)` | 设置品牌配置 |
| `initBrand()` | 初始化品牌（启动时调用，从服务端同步） |
| `VERSION` | 当前版本号（构建时通过 `VITE_APP_VERSION` 注入） |

---

## 版本号机制

- 开发时：`VERSION` 使用 `brand.ts` 中的硬编码回退值
- 构建时：Rust 通过环境变量 `VITE_APP_VERSION` 注入版本号给 Vite 生产构建
- 这样开发时不会触发 HMR 页面刷新

---

## 推荐 IDE

[VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
