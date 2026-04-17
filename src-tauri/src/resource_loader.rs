//! 资源文件加解密 / 缓存机制
//!
//! ## 背景
//! 产品经常需要打包第三方可执行文件（如 `ffmpeg.exe`、`yt-dlp.exe`），如果直接放
//! `src-tauri/resources/` 目录，installer 安装后这些 `.exe` 会裸露在程序目录下，
//! 同行解压 installer 就能一眼看穿技术栈、甚至直接拿走使用。
//!
//! ## 方案
//! 打包时把原始 `.exe` 加密为 `.dat` 文件放 `resources/`；程序运行时解密到
//! `%LOCALAPPDATA%\<bundle-id>\cache\` 下并改回 `.exe` 后缀执行。
//! installer 可见 = 加密 `.dat`（同行光有文件无法直接用），
//! 程序目录可见 = 只有 `.dat`，用户目录缓存 = 实际 `.exe`（隐蔽位置）。
//!
//! ## 文件格式
//! ```text
//! [4 bytes magic = "QSR1"]
//! [4 bytes u32 (little-endian) = payload length]
//! [N bytes payload = XOR-encrypted raw bytes]
//! ```
//!
//! ## 加密算法
//! XOR 密钥轮转。不是强加密，但足以让同行无法"复制 .dat 换个名就能跑"，且
//! 不引入额外的 crypto 依赖（否则 Rust 二进制体积膨胀 + 编译慢）。需要更强
//! 保密时可升级为 AES-128-CTR（需加 `aes` crate）。
//!
//! ## 使用方式
//! 1. 产品在 `app_config.rs` 或别处定义一个稳定的密钥常量
//! 2. 一次性用 `scripts/encrypt-resource.mjs` 把原始 .exe 加密成 .dat
//! 3. 从 git 里移除原始 .exe，只保留 .dat 在 `resources/`
//! 4. 产品代码里用 `ensure_decrypted(app, "mcore.dat", "xvf.exe", KEY)` 拿到可执行路径

use std::fs;
use std::path::PathBuf;
use tauri::Manager;

const MAGIC: &[u8; 4] = b"QSR1";

/// XOR 加密（对称操作，加解密同一函数调用）。
fn xor_bytes(data: &mut [u8], key: &[u8]) {
    if key.is_empty() {
        return;
    }
    for (i, b) in data.iter_mut().enumerate() {
        *b ^= key[i % key.len()];
    }
}

/// 把原始字节加密成 `.dat` 格式（主要供脚本/测试使用，运行时一般不会调用）。
pub fn encrypt_bytes(raw: &[u8], key: &[u8]) -> Vec<u8> {
    let mut payload = raw.to_vec();
    xor_bytes(&mut payload, key);
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

/// 从 `.dat` 格式解密出原始字节。
pub fn decrypt_bytes(dat: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    if dat.len() < 8 {
        return Err("加密资源文件过短，已损坏".into());
    }
    if &dat[..4] != MAGIC {
        return Err("加密资源文件魔数不匹配，非有效 QSR1 格式".into());
    }
    let len = u32::from_le_bytes([dat[4], dat[5], dat[6], dat[7]]) as usize;
    if dat.len() < 8 + len {
        return Err(format!(
            "加密资源文件长度不匹配（期望 {} 字节 payload，实际 {} 字节）",
            len,
            dat.len() - 8
        ));
    }
    let mut payload = dat[8..8 + len].to_vec();
    xor_bytes(&mut payload, key);
    Ok(payload)
}

/// 应用的"运行时缓存目录"：`%LOCALAPPDATA%\<bundle-id>\cache\`（Windows）
/// 或对应平台的 local_data_dir 子目录。
///
/// 选这里而不是 `%APPDATA%` 是因为 LocalAppData 不会被漫游同步，放大文件更合适。
fn app_runtime_cache_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // Tauri 2: local_data_dir 在 Windows 下 = %LOCALAPPDATA%
    let base = app
        .path()
        .local_data_dir()
        .map_err(|e| format!("获取 local_data_dir 失败: {}", e))?;
    let bundle = app.config().identifier.clone();
    let dir = base.join(bundle).join("cache");
    fs::create_dir_all(&dir).map_err(|e| format!("创建缓存目录失败 ({}): {}", dir.display(), e))?;
    Ok(dir)
}

/// 核心入口：确保加密资源被解密到用户缓存目录，返回可直接执行的路径。
///
/// - `encrypted_name`: `resources/` 下的加密文件名，如 `"mcore.dat"`
/// - `output_name`: 缓存目录下的输出名，建议用混淆名（如 `"xvf.exe"`），便于进一步
///   降低被同行一眼看穿的概率。扩展名决定用户双击能否执行。
/// - `key`: 解密密钥（建议每个产品自己定义，不要跨产品共用）
///
/// 行为：
/// 1. 如果缓存目录已有 `output_name` 且大小 > 0 → 直接返回（后续启动零开销）
/// 2. 否则读 `resources/<encrypted_name>` → 解密 → 写入缓存 → 返回路径
///
/// 失败情况都返回 `Err(String)`，调用方自行决定降级策略（比如回退到 PATH 查找）。
pub fn ensure_decrypted(
    app: &tauri::AppHandle,
    encrypted_name: &str,
    output_name: &str,
    key: &[u8],
) -> Result<PathBuf, String> {
    let cache_dir = app_runtime_cache_dir(app)?;
    let cached = cache_dir.join(output_name);

    // 已存在有效缓存 → 直接复用
    if let Ok(meta) = fs::metadata(&cached) {
        if meta.len() > 0 {
            return Ok(cached);
        }
    }

    // 从 resources 目录读加密文件
    let res_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("获取 resource_dir 失败: {}", e))?;
    // Tauri 2 打包后 resource_dir() 指向应用根目录，外层要再拼 `resources/`。
    // 某些路径下 resource_dir 本身就带 \\?\ 前缀，用 strip_prefix 处理一下。
    let dat_path = res_dir.join("resources").join(encrypted_name);
    let dat_bytes = fs::read(&dat_path).map_err(|e| {
        format!(
            "读取加密资源失败 ({}): {}（installer 可能没有打包该 .dat 文件）",
            dat_path.display(),
            e
        )
    })?;

    // 解密
    let raw = decrypt_bytes(&dat_bytes, key)?;

    // 原子写入：先写 tmp，再 rename。避免并发启动时写入一半被另一个进程读到残缺文件。
    let tmp = cached.with_extension("part");
    fs::write(&tmp, &raw)
        .map_err(|e| format!("写入缓存文件失败 ({}): {}", tmp.display(), e))?;
    fs::rename(&tmp, &cached)
        .map_err(|e| format!("缓存文件 rename 失败 ({} → {}): {}", tmp.display(), cached.display(), e))?;

    // Unix 需要 chmod +x，Windows 不需要
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&cached) {
            let mut perm = meta.permissions();
            perm.set_mode(0o755);
            let _ = fs::set_permissions(&cached, perm);
        }
    }

    Ok(cached)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_is_symmetric() {
        let key = b"test_key_2026";
        let raw = b"Hello, World! \xef\xbc\x81\xe4\xb8\xad\xe6\x96\x87";
        let encrypted = encrypt_bytes(raw, key);
        let decrypted = decrypt_bytes(&encrypted, key).unwrap();
        assert_eq!(&decrypted[..], &raw[..]);
    }

    #[test]
    fn decrypt_rejects_bad_magic() {
        let bad = b"XXXX\x00\x00\x00\x00";
        assert!(decrypt_bytes(bad, b"any").is_err());
    }

    #[test]
    fn decrypt_rejects_truncated() {
        let key = b"k";
        let mut enc = encrypt_bytes(b"abcdef", key);
        enc.truncate(10);
        assert!(decrypt_bytes(&enc, key).is_err());
    }
}
