# 青拾全网解析 (qingshi-khd-quanwangjiexi)

基于「青拾客户端模板」开发的全网视频解析下载工具。

支持抖音、快手、B站等主流平台的视频解析与无水印下载。

## 技术栈

- **桌面框架**: Tauri 2（Rust 后端）
- **前端框架**: Vue 3（Composition API + `<script setup>`）
- **UI 组件库**: PrimeVue 4 / PrimeIcons
- **状态管理**: Pinia
- **路由**: Vue Router 4
- **构建工具**: Vite 6
- **语言**: TypeScript + Rust

---

## 产品功能

| 功能 | 说明 |
|------|------|
| 视频解析 | 粘贴链接，自动识别平台并解析视频信息 |
| 批量下载 | 支持批量解析、筛选、一键下载 |
| 无水印下载 | 获取无水印原始视频 |
| 下载管理 | 解析中 / 下载中 / 下载失败 / 已完成 四个标签页 |

### 继承自基础框架的功能

| 功能 | 说明 |
|------|------|
| 用户认证 | 登录 / 注册 / 找回密码 / 解绑设备 |
| 充值系统 | 卡密充值 + 在线支付（微信/支付宝） |
| 品牌系统 | 多套 UI 模板，品牌配置从服务端同步 |
| 版本管理 | 检查更新 / 下载安装 |
| 窗口管理 | 自定义标题栏 / 托盘菜单 / 开机自启 |

---

## 项目结构

```
├── src/
│   ├── app/                      # [产品专属] 业务代码
│   │   ├── pages/
│   │   │   └── VideoDownloadView.vue   # 视频下载主页面
│   │   └── routes.ts             # 业务路由
│   ├── api/                      # [基础框架] 通用 API
│   ├── composables/              # [基础框架] 组合式函数
│   ├── templates/                # [基础框架] UI 模板
│   ├── stores/                   # [基础框架] 状态管理
│   ├── utils/                    # [基础框架] 工具函数
│   └── brand.ts                  # [基础框架] 品牌配置
├── src-tauri/
│   └── src/
│       ├── app_config.rs         # [产品专属] APP_ID / APP_KEY
│       └── lib.rs                # [基础框架] Rust 核心命令
├── .env.example                  # 环境变量示例
└── .env                          # 环境变量（不提交到 Git）
```

---

## 开发命令

```bash
# 安装依赖
npm install

# 启动开发模式
npm run tauri dev

# TypeScript 类型检查
npx vue-tsc --noEmit

# 构建生产版本
npm run tauri build
```

---

## 同步基础框架更新

本项目通过 `upstream` 远程仓库关联模板：

```bash
# 拉取模板最新代码
git fetch upstream

# 合并到当前分支
git merge upstream/master
```

---

## 远程仓库

| Remote | 地址 | 用途 |
|--------|------|------|
| `origin` | qingshi-khd-quanwangjiexi | 产品仓库，日常推送 |
| `upstream` | qingshi-khd-muban | 模板仓库，拉取框架更新 |
