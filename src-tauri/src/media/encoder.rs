//! 编码器自动优选（encoder）—— TODO，未来实现。
//!
//! 按用户机器硬件自动选最优 H.264 / H.265 编码器：
//! 1. NVIDIA 独显 → `h264_nvenc` / `hevc_nvenc`
//! 2. Intel 核显 → `h264_qsv` / `hevc_qsv`
//! 3. AMD 独显 → `h264_amf` / `hevc_amf`
//! 4. 都没有 → OpenH264（H.264 软编）/ 不支持 H.265 软编（LGPL-only 约束）
//!
//! 实现要点：
//! - 启动时探测：`avcodec_find_encoder_by_name("h264_nvenc")` 是否返回 Some
//! - 再用一个最小编码示例测试该 encoder 能否真实打开（有些机器装了 NVIDIA 驱动但
//!   GPU 不支持 NVENC，find 成功但 open 失败）
//! - 结果缓存到全局 OnceLock，后续直接复用

use super::MediaError;

/// 编码目标格式
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum VideoCodec {
    H264,
    H265,
}

/// 选出的编码器名字（给 `avcodec_find_encoder_by_name` 用）
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SelectedEncoder {
    pub name: String,
    pub is_hardware: bool,
}

/// 自动探测并返回可用的最优编码器。
///
/// TODO: 未实现。见 media/encoder.rs TODO。
#[allow(dead_code, unused_variables)]
pub fn pick_encoder(codec: VideoCodec) -> Result<SelectedEncoder, MediaError> {
    Err(MediaError::other(
        "编码器选择器尚未实现，见 media/encoder.rs TODO",
    ))
}
