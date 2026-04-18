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
#[derive(Debug, Clone)]
pub struct SelectedEncoder {
    pub name: String,
    pub is_hardware: bool,
}

static CACHED_H264: OnceLock<Option<SelectedEncoder>> = OnceLock::new();
static CACHED_H265: OnceLock<Option<SelectedEncoder>> = OnceLock::new();

/// 自动探测并返回可用的最优编码器。结果会缓存，多次调用不重复探测。
pub fn pick_encoder(codec: VideoCodec) -> Result<SelectedEncoder, MediaError> {
    super::ensure_init();

    let cache = match codec {
        VideoCodec::H264 => &CACHED_H264,
        VideoCodec::H265 => &CACHED_H265,
    };

    let result = cache.get_or_init(|| detect_best_encoder(codec));

    match result {
        Some(enc) => Ok(enc.clone()),
        None => Err(MediaError::other(format!(
            "未找到可用的 {:?} 编码器（请检查 GPU 驱动或 FFmpeg 编译选项）",
            codec
        ))),
    }
}

fn detect_best_encoder(codec: VideoCodec) -> Option<SelectedEncoder> {
    let candidates: &[(&str, bool)] = match codec {
        VideoCodec::H264 => &[
            ("h264_nvenc", true),
            ("h264_qsv", true),
            ("h264_amf", true),
            ("libopenh264", false),
        ],
        VideoCodec::H265 => &[
            ("hevc_nvenc", true),
            ("hevc_qsv", true),
            ("hevc_amf", true),
        ],
    };

    for &(name, is_hw) in candidates {
        if test_encoder(name) {
            return Some(SelectedEncoder {
                name: name.to_string(),
                is_hardware: is_hw,
            });
        }
    }

    None
}

/// 测试编码器是否真正可用（不只是 find 到，还要能 open）
fn test_encoder(name: &str) -> bool {
    let encoder = match ffmpeg::encoder::find_by_name(name) {
        Some(e) => e,
        None => return false,
    };

    let video_encoder = match encoder.video() {
        Ok(e) => e,
        Err(_) => return false,
    };

    let ctx = match ffmpeg::codec::context::Context::new() {
        ctx => ctx,
    };

    // 用最小参数尝试打开编码器
    {
        let mut enc = match ctx.encoder().video() {
            Ok(e) => e,
            Err(_) => return false,
        };

        enc.set_width(64);
        enc.set_height(64);
        enc.set_time_base(ffmpeg::Rational::new(1, 30));
        enc.set_format(ffmpeg::format::Pixel::YUV420P);

        match enc.open_as_with(video_encoder, ffmpeg::Dictionary::new()) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

/// 返回当前系统可用的编码器信息（供 UI 展示）
pub fn get_encoder_info() -> String {
    super::ensure_init();

    let h264 = detect_best_encoder(VideoCodec::H264);
    let h265 = detect_best_encoder(VideoCodec::H265);

    format!(
        "H.264: {} | H.265: {}",
        h264.map(|e| format!("{} ({})", e.name, if e.is_hardware { "GPU" } else { "CPU" }))
            .unwrap_or_else(|| "无".into()),
        h265.map(|e| format!("{} ({})", e.name, if e.is_hardware { "GPU" } else { "CPU" }))
            .unwrap_or_else(|| "无".into()),
    )
}
