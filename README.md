# 青拾客户端模板 (qingshi-khd-muban)

桌面客户端基础框架模板。各产品（视频下载、AI 视频、剪辑等）基于此模板扩展业务，通过 `git upstream` 机制同步基础框架更新。

## 模板提供的能力

### 通用基础设施

| 模块 | 说明 |
|------|------|
| 用户体系 | 登录 / 注册 / 找回密码 / 设备绑定 / 解绑 |
| 充值系统 | 卡密充值 + 在线支付（微信 / 支付宝） |
| 品牌系统 | 多套 UI 模板（green / orange / dark）+ 品牌配置从服务端加密同步 |
| 版本管理 | 检查更新 / 下载安装包 / 退出登录避免冲突 |
| 窗口管理 | 自定义标题栏 / 系统托盘 / 开机自启 / 单实例锁 |
| 会话保护 | 异常退出残留会话清理、session 持久化 |

### 反同行白嫖能力（重要）

| 模块 | 防御目标 |
|------|---------|
| 资源加密（`src-tauri/src/resource_loader.rs`） | 第三方 exe（yt-dlp 等）以加密 `.dat` 形式打包，不裸露在 installer 里。详见 [docs/resource-encryption.md](docs/resource-encryption.md) |
| libav 静态链接（`src-tauri/src/media/`） | ffmpeg 直接编进主程序，外部监控（Procmon / WMI）抓不到任何命令行参数。详见 [docs/libav-integration.md](docs/libav-integration.md) |

## 技术栈

- **桌面框架**：Tauri 2（Rust 后端）
- **前端**：Vue 3（Composition API + `<script setup>`）+ TypeScript
- **UI**：PrimeVue 4 + PrimeIcons
- **状态**：Pinia
- **路由**：Vue Router 4
- **构建**：Vite 6
- **音视频**：libav (FFmpeg 8.1) 静态链接，feature-gated

## 项目结构

```
client_muban/
├── src/
│   ├── api/                       # [基础] 通用 API 封装（auth / brand / pay / version）
│   ├── components/                # [基础] 公共组件（TitleBar 等）
│   ├── composables/               # [基础] 通用组合式函数
│   ├── stores/                    # [基础] Pinia 状态管理
│   ├── utils/                     # [基础] 工具函数（HMAC 签名、request）
│   ├── templates/                 # [基础] UI 模板（green / orange / dark）
│   ├── layouts/                   # [基础] 布局组件
│   ├── router/                    # [基础] 核心路由（合并业务路由）
│   ├── dev/                       # [基础] 开发工具（BrandManager / VersionManager）
│   ├── styles/                    # [基础] 全局样式
│   ├── brand.ts                   # [基础] 品牌配置系统
│   └── app/                       # [产品] 业务代码（pages / routes）
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                 # [基础] Rust 核心命令入口
│   │   ├── main.rs                # [基础] Tauri 启动
│   │   ├── database.rs            # [基础] SQLite 持久化
│   │   ├── resource_loader.rs     # [基础] 资源加密/缓存
│   │   ├── media/                 # [基础] libav 视频处理（feature = "media"）
│   │   │   ├── concat.rs          # 已实现：视频拼接 remux
│   │   │   ├── trim.rs            # TODO：时间裁剪
│   │   │   ├── filter.rs          # TODO：滤镜（crop/水印/字幕）
│   │   │   ├── merge.rs           # TODO：多流合并
│   │   │   ├── extract.rs         # TODO：抽音/视频轨
│   │   │   └── encoder.rs         # TODO：编码器自动优选
│   │   └── app_config.rs          # [产品] APP_ID / APP_KEY / RESOURCE_KEY
│   ├── resources/                 # 打包资源（加密 .dat / brand_config.enc 等）
│   └── tauri.conf.json
├── docs/
│   ├── libav-integration.md       # libav 集成完整指南
│   └── resource-encryption.md     # 资源加密机制使用文档
├── scripts/
│   └── encrypt-resource.mjs       # 资源加密脚本（一次性使用）
├── .env.example
├── package.json
└── README.md
```

**修改约定**（按 `.cursor/rules/project-structure.mdc`）：

- **产品专属代码** 只放 `src/app/`、`src-tauri/src/app_config.rs`
- **基础框架代码** 不要在产品仓库里改，改了模板再 `git pull upstream master` 同步
- 业务秘密（如视频处理的具体配方参数）只放产品仓库，模板只提供无业务语义的通用零件

## 基于模板新建产品

### 步骤 1：克隆模板为新产品

```bash
git clone https://github.com/Zander-zhq/qingshi-khd-muban.git my-new-product
cd my-new-product
git remote rename origin upstream
git remote add origin https://github.com/<your-org>/<your-new-repo>.git
git push -u origin master
```

### 步骤 2：环境前置

#### 2.1 装 vcpkg + ffmpeg（如果产品要用 libav 视频处理）

详见 [docs/libav-integration.md](docs/libav-integration.md)。简版：

```powershell
git clone https://github.com/Microsoft/vcpkg.git E:\vcpkg
E:\vcpkg\bootstrap-vcpkg.bat -disableMetrics

# 复制 custom triplet（只编 Release，跳过 Debug）
# 内容见 docs/libav-integration.md
# 路径：E:\vcpkg\triplets\community\x64-windows-static-release.cmake

E:\vcpkg\vcpkg.exe install ffmpeg:x64-windows-static-release
```

设环境变量：

```powershell
[Environment]::SetEnvironmentVariable("VCPKG_ROOT", "E:\vcpkg", "User")
[Environment]::SetEnvironmentVariable("FFMPEG_DIR", "E:\vcpkg\installed\x64-windows-static-release", "User")
[Environment]::SetEnvironmentVariable("VCPKG_DEFAULT_TRIPLET", "x64-windows-static-release", "User")
```

#### 2.2 装 LLVM（bindgen 依赖，libav 必需）

从 https://github.com/llvm/llvm-project/releases 下 Windows 安装包，默认装 `C:\Program Files\LLVM\`。

#### 2.3 装 Node 依赖

```bash
npm install
```

### 步骤 3：填产品配置

`src-tauri/src/app_config.rs`：

```rust
pub const APP_ID: &str = "<服务端分配的产品 ID>";
pub const APP_KEY: &str = "<服务端分配的密钥>";

// resource_key() 自动复用 APP_ID + APP_KEY，一般不用改
pub fn resource_key() -> Vec<u8> {
    format!("{}.{}", APP_ID, APP_KEY).into_bytes()
}
```

### 步骤 4：业务代码

在 `src/app/pages/` 下写页面，在 `src/app/routes.ts` 注册路由（自动注入到 `/main` 下的子路由）：

```typescript
const appRoutes: RouteRecordRaw[] = [
  {
    path: 'my-feature',
    name: 'MyFeature',
    component: () => import('./pages/MyFeatureView.vue'),
    meta: { title: `功能名 - ${brand.brand_name}`, requiresAuth: true },
  },
]
```

### 步骤 5：启用 media 模块（如需视频处理）

在产品 `src-tauri/Cargo.toml` 加：

```toml
[dependencies.ffmpeg-next]
version = "8.1"
default-features = false
features = ["codec", "format", "software-resampling", "software-scaling"]
```

然后业务代码 `use crate::media::concat_remux;` 即可调用。详见 [docs/libav-integration.md](docs/libav-integration.md)。

### 步骤 6：加密第三方 exe（如打包了 yt-dlp 等）

详见 [docs/resource-encryption.md](docs/resource-encryption.md)。简版：

```powershell
$env:QSR_KEY = "<APP_ID>.<APP_KEY>"  # 与 Rust 端 resource_key() 一致
node scripts/encrypt-resource.mjs `
  src-tauri/resources/yt-dlp.exe `
  src-tauri/resources/mcore.dat
git rm --cached src-tauri/resources/yt-dlp.exe  # 原始 exe 不入 git
```

## 开发命令

```bash
# 启动开发模式
npm run tauri dev

# TypeScript 类型检查
npx vue-tsc --noEmit

# 构建生产版本
npm run tauri build

# 模板 Rust 单元测试（含 media 模块）
cd src-tauri && cargo test --features media

# 不启用 media（纯前端 / 不需视频处理的产品）
cd src-tauri && cargo build  # 默认不启用 media，不依赖 vcpkg
```

## 同步基础框架更新

模板更新后，所有产品执行：

```bash
git fetch upstream
git merge upstream/master
# 解冲突（一般只在产品同时改了基础文件时出现），然后
git push origin master
```

## 远程仓库（产品仓库的标准配置）

| Remote | 地址 | 用途 |
|--------|------|------|
| `origin` | 产品自己的 git 仓库 | 日常推送 |
| `upstream` | qingshi-khd-muban | 拉取模板更新（push 默认禁用） |

## Cursor / VS Code 集成

`.cursor/rules/project-structure.mdc` 内置了项目结构规范。Cursor / Claude Code 在编辑时会遵循"产品代码只改 `src/app/` + `app_config.rs`"等约定。
