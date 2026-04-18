//! 多流合并（merge）—— TODO，未来实现。
//!
//! 相当于 `ffmpeg -i v.mp4 -i voice.aac -map 0:v -map 0:a -map 1:a \
//!     -c copy -disposition:a:0 default -metadata:s:a:1 title="配音" out.mp4`
//!
//! 用途：把多个源的流合并到一个输出容器。未来产品侧用它 + 具体的
//! metadata/disposition 参数组合，拼出"双轨道过审"这类业务配方。
//!
//! 实现要点：
//! - 接受 `Vec<StreamSpec>`：每个 spec 指定"来自哪个文件的哪一路流、metadata、disposition"
//! - 遍历 specs 把对应流加到输出容器
//! - 读每个源的 packet → rescale → write
//! - 支持同时 remux 多个输入（类似 concat 但跨文件跨流类型）

use std::path::PathBuf;
use std::collections::HashMap;

use super::MediaError;

/// 一个"要合并到输出的流"的规格。通用零件，无业务含义。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StreamSpec {
    /// 来源文件
    pub source_file: PathBuf,
    /// 来源文件里的 stream index（用 `ffprobe` 或本模块未来提供的探测 API 拿）
    pub source_stream_index: usize,
    /// metadata 键值对（如 `title="原声"`、`language="zho"`）
    pub metadata: HashMap<String, String>,
    /// 是否设为默认流（对应 `-disposition:a:N default`）
    pub is_default: bool,
}

/// 合并多个流到一个输出。各 spec 会按 `Vec` 顺序作为输出的 stream[0], stream[1]...
///
/// TODO: 未实现。见 media/merge.rs TODO。
#[allow(dead_code, unused_variables)]
pub fn merge_streams(
    specs: &[StreamSpec],
    output: &std::path::Path,
) -> Result<u64, MediaError> {
    Err(MediaError::other("merge 尚未实现，见 media/merge.rs TODO"))
}
