# 资源加密机制（防同行分析）

## 背景

Tauri 应用经常需要打包第三方可执行文件（`ffmpeg.exe`、`yt-dlp.exe` 等）。如果直接放 `src-tauri/resources/`，installer 安装后 `.exe` 会裸露在用户的程序目录下，**同行解压 installer 就能一眼看穿技术栈，甚至直接拿来用**。

本机制通过"**加密打包 + 用户目录延迟解压**"的方式把这些文件隐藏起来：

| 位置 | 用户能看到什么 |
|---|---|
| installer 内部 | 乱七八糟的 `.dat` 文件（无法直接运行） |
| 安装目录 `C:\Program Files\<App>\resources\` | 同上（只有 `.dat`） |
| 运行时释放位置 `%LOCALAPPDATA%\<bundle-id>\cache\` | 解密后的 `.exe`（隐蔽位置，普通用户找不到） |

## 工作流

### 一次性：加密原始 exe

在产品项目根目录下，对每个需要保护的文件执行：

```powershell
# Windows PowerShell
$env:QSR_KEY = "my_product_unique_key_2026"

node scripts/encrypt-resource.mjs `
  src-tauri/resources/yt-dlp.exe `
  src-tauri/resources/mcore.dat

node scripts/encrypt-resource.mjs `
  src-tauri/resources/ffmpeg.exe `
  src-tauri/resources/media.dat
```

脚本会在 `resources/` 下生成加密后的 `.dat` 文件（大小和原始文件几乎一样）。

### 一次性：从 git 里移除原始 exe

```powershell
git rm --cached src-tauri/resources/yt-dlp.exe
git rm --cached src-tauri/resources/ffmpeg.exe
```

然后在 `.gitignore` 里加（防止以后误提交）：

```gitignore
src-tauri/resources/*.exe
```

只把 `.dat` 提交到仓库。

### Rust 端调用：改 find_* 函数

原来的 `find_ytdlp` / `find_ffmpeg` 从 `resources/*.exe` 找文件，改成调用模板提供的 `ensure_decrypted` helper：

```rust
use qingshi_khd_muban_lib::resource_loader::ensure_decrypted;

// 产品的 app_config.rs 里定义（每个产品自己的密钥，别复用！）
pub const RESOURCE_KEY: &[u8] = b"my_product_unique_key_2026";

fn find_ytdlp(app: Option<&tauri::AppHandle>) -> Option<std::path::PathBuf> {
    let app = app?;
    // 从加密 .dat 解密到 %LOCALAPPDATA%\<bundle>\cache\xvf.exe
    ensure_decrypted(app, "mcore.dat", "xvf.exe", crate::app_config::RESOURCE_KEY)
        .ok()
        .or_else(|| {
            // 降级：仍然尝试 PATH 里的 yt-dlp，兼容开发场景
            std::process::Command::new("where").arg("yt-dlp").output().ok()
                .filter(|o| o.status.success())
                .and_then(|o| {
                    String::from_utf8_lossy(&o.stdout).lines().next()
                        .map(std::path::PathBuf::from)
                })
        })
}
```

命名策略：**把缓存文件名起得尽量无意义**（`xvf.exe`、`mcm.exe`、`core3.exe` 等），这样同行即使翻到了 `%LOCALAPPDATA%\<bundle>\cache\` 也不容易一眼判断是什么工具。

## 密钥管理

### 每个产品独立密钥

**不要**所有产品共用同一把密钥。密钥建议定义在产品自己的 `src-tauri/src/app_config.rs` 里：

```rust
// app_config.rs（产品专属代码，不会被模板 upstream 覆盖）
pub const RESOURCE_KEY: &[u8] = b"视频下载_qingshi_vd_2026_\xe7\xa7\x81\xe9\x92\xa5";
```

这样同行即使破解了 A 产品的密钥，换到 B 产品还得重新破一次，提高单点泄露的防御成本。

### 密钥长度

XOR 轮转的安全性大致正比于密钥长度。建议密钥 ≥ 32 字节，含 ASCII + 非 ASCII 字节（比如中文 UTF-8 字节）。

### 密钥变更

需要轮换密钥时：
1. 修改产品的 `RESOURCE_KEY` 常量
2. 重新运行 `scripts/encrypt-resource.mjs` 生成新的 `.dat`
3. 提交新 `.dat` 到 git
4. 下次打包自动生效（旧版本用户的缓存 `.exe` 仍能继续用，因为解密在首次安装时完成）

## 算法细节

### 文件格式

```
Offset  Size  Content
------  ----  -------
  0      4    Magic = "QSR1"（ASCII: 51 53 52 31）
  4      4    Payload length, u32 little-endian
  8      N    Payload = XOR(原始字节, 密钥循环)
```

### 加密强度

XOR 轮转加密**不是**强加密（同行拿到 Rust 二进制 + 一个 .dat 文件 + 原始 exe 前几字节就能反推密钥）。它的定位是：

- ✅ 挡住"下载 installer → 解压 → 翻资源目录"的同行（95% 的场景）
- ✅ 挡住"改名 .dat → .exe 直接跑"的尝试
- ❌ 挡不住"反编译 Rust 二进制 + 知道原始 exe 头部字节的逆向分析师"

需要更强保密时可升级为 AES-128-CTR（需加 `aes` crate，二进制多 ~100KB）。

## 常见问题

**Q: 开发模式（`npm run tauri dev`）下怎么办？**
A: 仍然需要先跑一次加密脚本生成 `.dat`，Rust 端 `ensure_decrypted` 用同样方式解压到缓存。开发和发布用同一套代码路径，避免行为差异。

**Q: 用户删了 `%LOCALAPPDATA%\<bundle>\cache\` 怎么办？**
A: 下次启动自动重新解密，无需用户干预。

**Q: 用户用 Everything 能搜到 `xvf.exe` 吗？**
A: 能，但在茫茫多 .exe 里不易判断是什么工具。如果需要完全隐蔽，可进一步改用 `hidden` 文件属性（`attrib +h`）或用 mmap 在内存里执行（实现复杂度高很多，不建议）。

**Q: 防病毒软件会误报吗？**
A: yt-dlp.exe 本身有时被误报（PyInstaller 打包特征）。解密后的 exe 仍是原版字节，误报概率不会变化。首次解压时写入用户目录可能被 AV 扫描（延迟几秒），之后缓存命中无扫描。

## 对应代码位置

- Rust 端 helper：`src-tauri/src/resource_loader.rs`
- 加密脚本：`scripts/encrypt-resource.mjs`
- 文档：`docs/resource-encryption.md`（本文件）
