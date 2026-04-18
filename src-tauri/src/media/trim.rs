//! 时间裁剪（trim）—— TODO，未来实现。
//!
//! 相当于 `ffmpeg -ss <start> -to <end> -i in.mp4 -c copy out.mp4`。
//! 按指定的起止时间裁出片段，不重编码。
//!
//! 实现要点：
//! - 输入时用 `av_seek_frame` 定位到 <start> 附近的关键帧
//! - 读 packet 时判断 pts：小于 start 或大于 end 的跳过
//! - 首 packet 的 pts 减去 start 让输出从 0 开始
//! - 所有流同步偏移

use std::path::Path;

use super::MediaError;

/// 按时间裁剪输入文件的 \[start, end\] 区间到输出。时间单位：秒。
///
/// TODO: 未实现。本次范围不做裁剪，仅保留骨架。
#[allow(dead_code, unused_variables)]
pub fn trim(
    input: &Path,
    output: &Path,
    start_sec: f64,
    end_sec: f64,
) -> Result<u64, MediaError> {
    Err(MediaError::other("trim 尚未实现，见 media/trim.rs TODO"))
}
