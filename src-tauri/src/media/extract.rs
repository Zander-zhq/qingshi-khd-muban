//! 抽取单流（extract）—— TODO，未来实现。
//!
//! 相当于 `ffmpeg -i in.mp4 -map 0:a -c copy out.aac`（抽音轨）
//! 或 `ffmpeg -i in.mp4 -map 0:v -c copy out.h264`（抽视频裸流）。
//!
//! 实现要点：很简单的 remux 子集，只挑一路流复制到输出。

use std::path::Path;

use super::MediaError;

/// 抽取输入里指定类型的第一路流到输出文件。
///
/// `stream_kind` 支持 `"audio"` 或 `"video"`。
///
/// TODO: 未实现。见 media/extract.rs TODO。
#[allow(dead_code, unused_variables)]
pub fn extract_stream(
    input: &Path,
    output: &Path,
    stream_kind: &str,
) -> Result<u64, MediaError> {
    Err(MediaError::other("extract 尚未实现，见 media/extract.rs TODO"))
}
