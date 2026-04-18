//! 滤镜处理（filter）—— TODO，未来实现。
//!
//! 相当于 `ffmpeg -i in.mp4 -vf "<filter_chain>" -c:v openh264 out.mp4`。
//! 支持 crop、scale、overlay、blur、drawtext（字幕/水印）等。
//!
//! 实现要点：
//! - 用 `ffmpeg_next::filter::Graph` 构建 filter chain
//! - 解码输入 → 送 filter graph → 从 sink 拉处理后的 frame
//! - 用 [`super::encoder`] 选出编码器，把处理后的 frame 编成 packet
//! - mux 到输出容器
//!
//! 会触发重编码（因为滤镜改变了像素数据），所以要依赖编码器选择器。

use std::path::Path;

use super::MediaError;

/// 对输入应用 filter 链（filter 表达式同 ffmpeg CLI 的 `-vf` 语法）。
///
/// TODO: 未实现。见 media/filter.rs TODO。
#[allow(dead_code, unused_variables)]
pub fn apply_filter(
    input: &Path,
    output: &Path,
    filter_expr: &str,
) -> Result<u64, MediaError> {
    Err(MediaError::other("filter 尚未实现，见 media/filter.rs TODO"))
}
