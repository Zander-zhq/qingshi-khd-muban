# libav 集成指南（替代 ffmpeg.exe 子进程）

## 为什么

- **安全**：早期版本用 `Command::new("ffmpeg.exe")` 启子进程，命令行参数会被 `Get-Process` / `Procmon` / `WMI` 监控到，同行能直接抄走视频处理配方。静态链接 libav 后所有处理在主进程内存里完成，没有新进程就没有可监控事件。
- **体积**：打包发行版不再需要 130 MB 的 `ffmpeg.exe`（或 130 MB 的 `media.dat` 加密件），主程序只多 15-30 MB。
- **性能**：省 100-300 ms 进程启动开销；多步链式处理时省中间文件磁盘 I/O。

## 架构分层

```
┌─ 产品层（client_shipinxiazai / 其他）─────────┐
│  业务组合（HLS 下载流程、过审配方...）         │
│  从模板 use crate::media::concat_remux 等     │
├─ 模板层（client_muban）────────────────────────┤
│  通用原子操作，无业务语义                       │
│  src-tauri/src/media/                          │
│    ├── concat.rs   视频拼接（已实现）           │
│    ├── trim.rs     时间裁剪（TODO）             │
│    ├── filter.rs   滤镜（TODO）                 │
│    ├── merge.rs    多流合并（TODO）             │
│    ├── extract.rs  抽流（TODO）                 │
│    └── encoder.rs  编码器选择器（TODO）         │
├─ libav 层（vcpkg 编译的静态库）────────────────┤
│  libavformat / libavcodec / libavfilter /      │
│  libavutil / libswscale / libswresample        │
└────────────────────────────────────────────────┘
```

**核心原则**：业务秘密（"双轨道过审"的具体参数组合）永远留产品仓库，模板只提供无业务语义的通用零件。

## 一次性环境搭建

### 1. 装 vcpkg（路径必须无中文无空格）

```powershell
# 任选一盘，这里以 E: 为例
git clone https://github.com/Microsoft/vcpkg.git E:\vcpkg
E:\vcpkg\bootstrap-vcpkg.bat -disableMetrics
```

### 2. 装 LLVM（bindgen 依赖）

从 https://github.com/llvm/llvm-project/releases 下载 Windows 安装包，默认路径 `C:\Program Files\LLVM\`。

### 3. 编译 ffmpeg 静态库

```powershell
E:\vcpkg\vcpkg.exe install ffmpeg:x64-windows-static --recurse
```

**首次编译约 30-60 分钟**，后续增量 1-2 分钟。成品在 `E:\vcpkg\installed\x64-windows-static\` 下。

许可证约束：默认 port 不启用 GPL 组件（libx264/libx265），产出纯 LGPL 静态库，可用于闭源商业软件。

### 4. 设置环境变量

让 `ffmpeg-next` 的 build.rs 知道去哪找 ffmpeg：

```powershell
[Environment]::SetEnvironmentVariable("VCPKG_ROOT", "E:\vcpkg", "User")
[Environment]::SetEnvironmentVariable("FFMPEG_DIR", "E:\vcpkg\installed\x64-windows-static", "User")
# 告诉 ffmpeg-next 用静态链接
[Environment]::SetEnvironmentVariable("FFMPEG_PKG_CONFIG_PATH", "E:\vcpkg\installed\x64-windows-static\lib\pkgconfig", "User")
```

重启 IDE / 终端让环境变量生效。

## 在产品里启用

### 步骤 1：Cargo.toml 加 feature

```toml
[dependencies]
# ... 其他依赖 ...

# 启用 media 模块（模板里已声明为 optional）
ffmpeg-next = { version = "7.1", default-features = false,
                features = ["codec", "format", "software-resampling",
                            "software-scaling", "build"] }
```

（直接加 `ffmpeg-next` 的声明即可，模板的 `media` feature 也会自动启用对应模块）

### 步骤 2：在业务代码里调用

```rust
use std::path::PathBuf;

// 产品的 Tauri 命令或业务函数
fn my_download_flow(ts_files: Vec<PathBuf>, output_mp4: &std::path::Path) 
    -> Result<u64, String> 
{
    // 直接调模板的通用原子
    crate::media::concat_remux(&ts_files, output_mp4)
        .map_err(|e| e.to_string())
}
```

### 步骤 3：在 async 上下文里包 spawn_blocking

libav 是同步 C API，不能直接在 async 函数里跑（会阻塞 tokio runtime）。用 `spawn_blocking`：

```rust
let size = tokio::task::spawn_blocking(move || {
    crate::media::concat_remux(&ts_files_clone, &output_clone)
}).await
    .map_err(|e| format!("spawn_blocking 失败: {}", e))?
    .map_err(|e| format!("合并失败: {}", e))?;
```

## 常见坑

### 1. `linking with link.exe failed` / `cannot open file 'avcodec.lib'`

**原因**：`FFMPEG_DIR` 环境变量没设或指错了。

**排查**：
```powershell
$env:FFMPEG_DIR
dir E:\vcpkg\installed\x64-windows-static\lib\avcodec.lib  # 应该存在
```

### 2. `bindgen` 报错找不到 `clang.dll`

**原因**：LLVM 没装或没加 PATH。

**排查**：
```powershell
Get-Command clang
# 如果找不到：把 C:\Program Files\LLVM\bin 加到 PATH
```

### 3. vcpkg 编译 ffmpeg 报 `meson` 或 `nasm` 缺失

vcpkg 会自动下载这些构建工具，如果失败通常是**网络问题**。重试一次或配代理。

### 4. 主 exe 体积从 ~15MB 涨到 ~30MB

**正常**。静态链接 libav 核心库（avcodec + avformat + avutil + swscale + swresample）约 15-20 MB。

### 5. `ffmpeg_next::init()` panic

**原因**：vcpkg 编译的 ffmpeg 版本和 `ffmpeg-next` crate 版本不匹配。

**解决**：锁定版本对应关系：
- `ffmpeg-next 7.1.x` ↔ `FFmpeg 7.x`（vcpkg 当前默认）
- `ffmpeg-next 6.x` ↔ `FFmpeg 6.x`

升级 vcpkg ffmpeg 或 `ffmpeg-next` 时务必同步。

## 未来扩展

当前只实现了 `concat_remux`。未来加功能的边际成本：

| 新原子 | 预计工作量 | 使用方 |
|---|---|---|
| `trim`（时间裁剪） | 1-2 天 | 剪辑/下载去头尾 |
| `extract_stream`（抽音/视频轨） | 0.5-1 天 | 分离音轨业务 |
| `merge_streams`（多流合并） | 3-5 天 | 双轨道过审、配音合成 |
| `apply_filter`（crop/水印/字幕） | 5-7 天 | 剪辑产品 |
| `encoder` 选择器 | 1-2 天 | filter/merge 依赖它 |

骨架文件都已建好（`src-tauri/src/media/{trim,filter,merge,extract,encoder}.rs`），实现时直接填函数体，**不用改架构**。

## 对比改造前后

| 维度 | ffmpeg.exe 子进程 | libav 静态链接 |
|---|---|---|
| 打包体积（installer） | 148 MB | ~80 MB |
| 主 exe 体积 | ~15 MB | ~30 MB |
| 启动开销（单次调用） | 100-300 ms | ~0 |
| 监控能抓到命令行 | 能 | 不能（没有子进程） |
| 许可证 | 无约束 | LGPL（文档列出即可合规） |
| 升级 ffmpeg | 换 exe 文件 | 重编 vcpkg（30-60 分钟） |
