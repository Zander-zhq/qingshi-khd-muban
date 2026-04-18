//! 媒体处理模块的统一错误类型。

use std::path::PathBuf;

/// `media` 模块各原子操作的错误类型。
///
/// 用统一错误类型避免每个原子自定义一套，产品侧匹配错误时更简单。
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// libav 底层错误（由 `ffmpeg_next::Error` 转来）
    #[error("libav 错误: {0}")]
    Libav(#[from] ffmpeg_next::Error),

    /// 输入文件缺失/无法打开
    #[error("输入文件不存在或无法打开: {0}")]
    InputNotFound(PathBuf),

    /// 输入列表为空
    #[error("操作需要至少一个输入文件")]
    EmptyInput,

    /// 输入文件之间不兼容（codec/分辨率/采样率等不匹配）
    #[error("输入文件不兼容: {0}")]
    IncompatibleInputs(String),

    /// 输入中找不到需要的流（如要抽音频但文件没音轨）
    #[error("输入中未找到所需的流: {0}")]
    StreamNotFound(String),

    /// 输出容器格式无法从文件后缀推断
    #[error("无法从输出路径推断容器格式: {0}")]
    UnknownOutputFormat(PathBuf),

    /// 文件系统 I/O 错误
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),

    /// 其他通用错误
    #[error("{0}")]
    Other(String),
}

impl MediaError {
    /// 方便从字面字符串构造 `Other`
    pub fn other(msg: impl Into<String>) -> Self {
        MediaError::Other(msg.into())
    }
}
