//! 视频处理模块（基于 libav 静态链接）
//!
//! ## 设计哲学
//! 本模块只暴露**无业务语义的通用原子操作**（remux、trim、filter 等）。
//! 各产品用这些原子**组合**出自己的业务逻辑（下载流程、过审配方等），
//! 业务秘密永远只留产品仓库，不进模板。
//!
//! ## 为什么不用 ffmpeg.exe 子进程
//! 早期版本通过 `Command::new("ffmpeg.exe")` 启动子进程调用 FFmpeg CLI，
//! 有两个大问题：
//!  1. **命令行参数可被外部监控**（Win32_Process.CommandLine / Procmon / WMI），
//!     辛苦测出的业务配方（如"双轨道过审"参数组合）会被同行白嫖
//!  2. 需要打包 ~130 MB 的 ffmpeg.exe，即便加密成 .dat，运行时仍要解密落盘
//!
//! 本模块改为**静态链接 libav 库**（libavformat/libavcodec/libavfilter 等）到
//! 主程序二进制。所有音视频处理在你进程内存里完成，不再启动任何子进程。
//! 外部监控工具从源头上抓不到任何参数。
//!
//! ## 许可证（LGPL-only，闭源商业软件合规）
//! vcpkg 编译 ffmpeg 时关闭 GPL 组件：`--disable-gpl --disable-nonfree`，
//! 避开 libx264/libx265/libfdk_aac 的传染性许可。静态链接的是纯 LGPL 组件：
//!  - libavformat / libavcodec / libavfilter / libavutil / libswscale / libswresample
//!
//! H.264 编码以后要用时走 OpenH264（Cisco 代付专利费）或 GPU 硬编（NVENC/QSV/AMF）。
//!
//! ## 模块组织
//!  - [`concat`] 视频拼接（多文件 remux，不重编码）
//!  - [`remux`]   容器转封装（TS→MP4 等，不重编码）
//!  - [`trim`]    时间裁剪（keyframe-based，不重编码）
//!  - [`silence`] 音频静音检测
//!  - [`filter`]  滤镜（TODO）
//!  - [`merge`]   多流合并（TODO）
//!  - [`extract`] 抽音轨/视频轨（TODO）
//!  - [`encoder`] 编码器选择器（OpenH264/NVENC/QSV/AMF 自动优选）（TODO）

mod error;
pub mod concat;
pub mod remux;
pub mod trim;
pub mod silence;
pub mod smart_cut;
pub mod filter;
pub mod merge;
pub mod extract;
pub mod encoder;

pub use error::MediaError;
pub use concat::concat_remux;
pub use remux::remux;
pub use remux::remux_url;
pub use trim::trim;
pub use silence::{detect_silence, SilenceRange};
pub use smart_cut::smart_cut;
pub use encoder::{pick_encoder, get_encoder_info, VideoCodec, SelectedEncoder};

/// 延迟初始化 libav 网络和日志。多次调用幂等。
///
/// 内部使用 `std::sync::Once`，无需担心并发。在任何 libav 操作前调用即可；
/// 各 atom 函数（`concat_remux` 等）已自动调用，业务层一般不需要直接用。
pub fn ensure_init() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = ffmpeg_next::init();
        // 启用网络协议支持（http/https/m3u8 等 URL 输入需要）
        unsafe { ffmpeg_next::ffi::avformat_network_init(); }
        ffmpeg_next::util::log::set_level(ffmpeg_next::util::log::Level::Error);
    });
}
