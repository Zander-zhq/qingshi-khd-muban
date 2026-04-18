//! 容器转封装（remux）
//!
//! 相当于 `ffmpeg -i input.ts -c copy -bsf:a aac_adtstoasc output.mp4`：
//! 把一个文件的音视频流**原样复制**到另一个容器格式，不重新编解码。
//!
//! 典型用途：
//! - TS → MP4（m3u8 下载的 TS 合并后转为 MP4）
//! - FLV → MP4
//! - MKV → MP4
//!
//! ## 与 `concat_remux` 的区别
//! - `concat_remux`：多个输入 → 一个输出（拼接）
//! - `remux`：一个输入 → 一个输出（换容器）
//!
//! ## 进度回调
//! `remux_with_progress` / `remux_url_with_progress` 允许传入 `FnMut(f64)` 回调，
//! 内部每处理 ~1% 进度或 500ms 触发一次（取二者较早），传入 0.0~1.0 的浮点。
//! 用于长耗时的 m3u8 下载场景，前端 UI 可实时显示下载进度。

use std::path::Path;
use std::time::{Duration, Instant};

use ffmpeg_next as ffmpeg;
use ffmpeg::{format, media, Rational};

use super::MediaError;

/// 空回调：不需要进度时传它
#[inline]
pub fn no_progress(_: f64) {}

/// 将单个输入文件转封装到指定输出格式（不重新编码）。
///
/// 无进度版本，内部等价于 `remux_with_progress(input, output, no_progress)`。
pub fn remux(input: &Path, output: &Path) -> Result<u64, MediaError> {
    remux_with_progress(input, output, no_progress)
}

/// 带进度回调版本。回调参数是 0.0~1.0 的浮点（当前位置 / 总时长）。
///
/// - `input`：输入文件路径（.ts / .flv / .mkv 等）
/// - `output`：输出文件路径，容器格式由后缀推断
/// - `on_progress`：进度回调，约每 1% 或 500ms 触发一次（取二者较早）
///
/// 注意：如果输入容器没有总时长信息（某些流式容器），回调参数会一直是 0.0 直到结束时 1.0。
pub fn remux_with_progress<F: FnMut(f64)>(
    input: &Path,
    output: &Path,
    on_progress: F,
) -> Result<u64, MediaError> {
    if !input.exists() {
        return Err(MediaError::InputNotFound(input.to_path_buf()));
    }
    remux_from_source(&input.to_string_lossy(), output, on_progress)
}

/// 从 URL 或文件路径转封装到指定输出格式（不重新编码）。
///
/// `source` 支持本地路径或 HTTP/HTTPS URL（如 m3u8 地址）。
pub fn remux_url(source: &str, output: &Path) -> Result<u64, MediaError> {
    remux_from_source(source, output, no_progress)
}

/// 带进度回调的 URL 版本。适合 m3u8 等网络流的"边下边处理"场景。
///
/// 回调在每处理一段音视频数据后触发（频率受限，约每 1% 或 500ms 取较早的触发一次），
/// 不会阻塞下载主流程。
pub fn remux_url_with_progress<F: FnMut(f64)>(
    source: &str,
    output: &Path,
    on_progress: F,
) -> Result<u64, MediaError> {
    remux_from_source(source, output, on_progress)
}

fn remux_from_source<F: FnMut(f64)>(
    source: &str,
    output: &Path,
    mut on_progress: F,
) -> Result<u64, MediaError> {
    super::ensure_init();

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut ictx = format::input(&source)?;
    let mut octx = format::output(&output)?;

    // 尝试获取输入总时长（AV_TIME_BASE 为单位，1_000_000 = 1 秒）。
    // 对 m3u8 输入，libav 解析 master playlist 时已经算出了所有分片累计时长。
    // 某些流式容器可能返回 0 / 负值，此时回调只在结束时触发。
    let total_duration_us = ictx.duration();
    let has_duration = total_duration_us > 0;

    let mut stream_mapping: Vec<Option<usize>> = vec![None; ictx.nb_streams() as usize];
    let mut out_time_bases: Vec<Rational> = Vec::new();
    let mut next_out_idx = 0usize;

    for in_stream in ictx.streams() {
        let medium = in_stream.parameters().medium();
        if medium != media::Type::Video && medium != media::Type::Audio {
            continue;
        }

        let mut out_stream = octx.add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::None))?;
        out_stream.set_parameters(in_stream.parameters());

        unsafe {
            (*out_stream.parameters().as_mut_ptr()).codec_tag = 0;
        }

        stream_mapping[in_stream.index()] = Some(next_out_idx);
        out_time_bases.push(Rational::new(0, 1));
        next_out_idx += 1;
    }

    if next_out_idx == 0 {
        return Err(MediaError::StreamNotFound("输入文件没有视频或音频流".into()));
    }

    octx.write_header()?;

    for (idx, tb_slot) in out_time_bases.iter_mut().enumerate() {
        let os = octx.stream(idx).ok_or_else(|| {
            MediaError::other(format!("output stream[{}] 获取失败", idx))
        })?;
        *tb_slot = os.time_base();
    }

    // ── 进度触发节流：避免每个 packet 都调回调（太频繁）──
    // 规则：进度涨幅 >= 1% 或距上次回调 >= 500ms，取较早触发
    let progress_step: f64 = 0.01; // 1%
    let min_interval = Duration::from_millis(500);
    let mut last_reported: f64 = 0.0;
    let mut last_time = Instant::now();
    on_progress(0.0); // 开始时先报 0，让 UI 从"排队中"立刻切换到"下载中"

    for (in_stream, mut packet) in ictx.packets() {
        let out_idx = match stream_mapping[in_stream.index()] {
            Some(i) => i,
            None => continue,
        };

        let in_tb = in_stream.time_base();
        let out_tb = out_time_bases[out_idx];

        // ── 算当前进度：用 packet 的 pts 换算成微秒，除以总时长 ──
        if has_duration {
            if let Some(pts) = packet.pts() {
                // pts * in_tb (秒) * 1_000_000 = 微秒
                let pts_us = pts * 1_000_000 * i64::from(in_tb.numerator())
                    / i64::from(in_tb.denominator()).max(1);
                let progress = (pts_us as f64 / total_duration_us as f64).clamp(0.0, 1.0);
                if progress - last_reported >= progress_step
                    || last_time.elapsed() >= min_interval
                {
                    on_progress(progress);
                    last_reported = progress;
                    last_time = Instant::now();
                }
            }
        }

        packet.rescale_ts(in_tb, out_tb);
        packet.set_position(-1);
        packet.set_stream(out_idx);
        packet.write_interleaved(&mut octx)?;
    }

    octx.write_trailer()?;

    // 最后确保报 100%
    on_progress(1.0);

    let size = std::fs::metadata(output)?.len();
    Ok(size)
}
