//! 编码器自动优选（encoder）
//!
//! 按用户机器硬件自动选最优 H.264 / H.265 编码器：
//! 1. NVIDIA 独显 → `h264_nvenc` / `hevc_nvenc`
//! 2. Intel 核显 → `h264_qsv` / `hevc_qsv`
//! 3. AMD 独显 → `h264_amf` / `hevc_amf`
//! 4. 都没有 → `libopenh264`（H.264 软编）
//!
//! 探测策略：
//! - `avcodec_find_encoder_by_name` 检查编码器是否编译进来
//! - 实际创建编码上下文并 open 测试（有些驱动装了但硬件不支持）
//! - 结果缓存到全局 OnceLock，后续直接复用

use std::sync::OnceLock;

use ffmpeg_next as ffmpeg;

use super::MediaError;

/// 编码目标格式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoCodec {
    H264,
    H265,
}

/// 选出的编码器
#[derive(Debug, Clone, serde::Serialize)]
pub struct SelectedEncoder {
    pub name: String,
    pub is_hardware: bool,
}

/// 单个编码器的探测结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct EncoderProbeResult {
    pub name: String,
    pub is_hardware: bool,
    pub found: bool,
    pub usable: bool,
    pub fail_reason: String,
}

/// 完整的编码器探测报告
#[derive(Debug, Clone, serde::Serialize)]
pub struct EncoderReport {
    pub selected: Option<SelectedEncoder>,
    pub probes: Vec<EncoderProbeResult>,
    pub suggestion: String,
}

static CACHED_H264: OnceLock<EncoderReport> = OnceLock::new();
static CACHED_H265: OnceLock<EncoderReport> = OnceLock::new();

/// 自动探测并返回可用的最优编码器。结果会缓存，多次调用不重复探测。
pub fn pick_encoder(codec: VideoCodec) -> Result<SelectedEncoder, MediaError> {
    let report = get_encoder_report(codec);
    match report.selected {
        Some(enc) => Ok(enc),
        None => Err(MediaError::other(report.suggestion.clone())),
    }
}

/// 获取完整的编码器探测报告（供前端 UI 展示）。结果会缓存。
pub fn get_encoder_report(codec: VideoCodec) -> EncoderReport {
    super::ensure_init();

    let cache = match codec {
        VideoCodec::H264 => &CACHED_H264,
        VideoCodec::H265 => &CACHED_H265,
    };

    cache.get_or_init(|| run_full_probe(codec)).clone()
}

fn run_full_probe(codec: VideoCodec) -> EncoderReport {
    let candidates: &[(&str, bool, &str)] = match codec {
        VideoCodec::H264 => &[
            ("h264_nvenc", true, "NVIDIA"),
            ("h264_qsv", true, "Intel"),
            ("h264_amf", true, "AMD"),
            ("libopenh264", false, "CPU"),
        ],
        VideoCodec::H265 => &[
            ("hevc_nvenc", true, "NVIDIA"),
            ("hevc_qsv", true, "Intel"),
            ("hevc_amf", true, "AMD"),
        ],
    };

    let mut probes: Vec<EncoderProbeResult> = Vec::new();
    let mut selected: Option<SelectedEncoder> = None;
    let mut driver_issues: Vec<String> = Vec::new();

    for &(name, is_hw, vendor) in candidates {
        let (found, usable, fail_reason) = test_encoder_detailed(name);

        probes.push(EncoderProbeResult {
            name: name.to_string(),
            is_hardware: is_hw,
            found,
            usable,
            fail_reason: fail_reason.clone(),
        });

        if usable && selected.is_none() {
            selected = Some(SelectedEncoder {
                name: name.to_string(),
                is_hardware: is_hw,
            });
        }

        if found && !usable && is_hw {
            driver_issues.push(format!(
                "{} 编码器 ({}) 已检测到但无法使用，可能是显卡驱动版本过低，建议更新 {} 显卡驱动",
                name, vendor, vendor
            ));
        }
    }

    let suggestion = if selected.is_some() {
        String::new()
    } else if !driver_issues.is_empty() {
        driver_issues.join("；")
    } else {
        "未找到可用的编码器，请更新显卡驱动或检查 FFmpeg 编译选项".into()
    };

    EncoderReport { selected, probes, suggestion }
}

/// 详细测试编码器：返回 (found, usable, fail_reason)
fn test_encoder_detailed(name: &str) -> (bool, bool, String) {
    let encoder = match ffmpeg::encoder::find_by_name(name) {
        Some(e) => e,
        None => return (false, false, "未编译进 FFmpeg".into()),
    };

    let video_encoder = match encoder.video() {
        Ok(e) => e,
        Err(e) => return (true, false, format!("不是视频编码器: {}", e)),
    };

    let ctx = ffmpeg::codec::context::Context::new();

    let mut enc = match ctx.encoder().video() {
        Ok(e) => e,
        Err(e) => return (true, false, format!("创建编码上下文失败: {}", e)),
    };

    enc.set_width(256);
    enc.set_height(256);
    enc.set_time_base(ffmpeg::Rational::new(1, 30));
    enc.set_format(ffmpeg::format::Pixel::YUV420P);

    match enc.open_as_with(video_encoder, ffmpeg::Dictionary::new()) {
        Ok(_) => (true, true, String::new()),
        Err(e) => (true, false, format!("初始化失败（可能是驱动版本过低）: {}", e)),
    }
}

/// 返回简洁的编码器信息字符串（供日志或简单 UI）
pub fn get_encoder_info() -> String {
    super::ensure_init();

    let h264 = get_encoder_report(VideoCodec::H264);
    let h265 = get_encoder_report(VideoCodec::H265);

    format!(
        "H.264: {} | H.265: {}",
        h264.selected
            .map(|e| format!("{} ({})", e.name, if e.is_hardware { "GPU" } else { "CPU" }))
            .unwrap_or_else(|| format!("无 - {}", h264.suggestion)),
        h265.selected
            .map(|e| format!("{} ({})", e.name, if e.is_hardware { "GPU" } else { "CPU" }))
            .unwrap_or_else(|| format!("无 - {}", h265.suggestion)),
    )
}
