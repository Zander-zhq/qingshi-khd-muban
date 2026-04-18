//! 时间裁剪（trim）
//!
//! 相当于 `ffmpeg -ss <start> -to <end> -i in.mp4 -c copy out.mp4`：
//! 按指定的起止时间裁出片段，不重新编解码。
//!
//! 实现策略：
//! - 用 `av_seek_frame` 定位到 start 附近的关键帧
//! - 读 packet 时跳过 pts < start 或 pts >= end 的
//! - 输出 packet 的 pts/dts 减去 start 偏移，让输出从 0 开始
//! - 所有流同步偏移
//!
//! ## 精度说明
//! 因为是 `-c copy` 不重编码，实际切割点只能在关键帧处。
//! 视频流的起始点可能比 `start_sec` 稍早（回退到前一个关键帧）。
//! 对于短剧分割场景，这种 1-2 秒的误差通常可以接受。

use std::path::Path;

use ffmpeg_next as ffmpeg;
use ffmpeg::{format, media, Rational};

use super::MediaError;

/// 按时间裁剪输入文件的 \[start_sec, end_sec\) 区间到输出。时间单位：秒。
///
/// - `input`：输入文件路径
/// - `output`：输出文件路径（容器格式由后缀推断）
/// - `start_sec`：起始时间（秒）
/// - `end_sec`：结束时间（秒），传 `f64::MAX` 表示到文件末尾
///
/// 返回：输出文件的字节数
pub fn trim(
    input: &Path,
    output: &Path,
    start_sec: f64,
    end_sec: f64,
) -> Result<u64, MediaError> {
    if !input.exists() {
        return Err(MediaError::InputNotFound(input.to_path_buf()));
    }
    if start_sec >= end_sec {
        return Err(MediaError::other("start_sec 必须小于 end_sec"));
    }

    super::ensure_init();

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut ictx = format::input(&input)?;
    let mut octx = format::output(&output)?;

    let mut stream_mapping: Vec<Option<usize>> = vec![None; ictx.nb_streams() as usize];
    let mut in_time_bases: Vec<Rational> = vec![Rational::new(0, 1); ictx.nb_streams() as usize];
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

        in_time_bases[in_stream.index()] = in_stream.time_base();
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

    // seek 到 start 位置附近的关键帧
    let start_ts = (start_sec * 1_000_000.0) as i64; // AV_TIME_BASE 单位
    ictx.seek(start_ts, ..start_ts)?;

    // 计算每个输入流的 start/end 对应的 pts 值
    let mut start_pts: Vec<i64> = Vec::new();
    let mut end_pts: Vec<i64> = Vec::new();
    for in_stream in ictx.streams() {
        let tb = in_stream.time_base();
        let s = sec_to_pts(start_sec, tb);
        let e = if end_sec >= f64::MAX / 2.0 { i64::MAX } else { sec_to_pts(end_sec, tb) };
        start_pts.push(s);
        end_pts.push(e);
    }

    // 记录每个输出流第一个 packet 的 pts，用于偏移输出从 0 开始
    let mut first_pts: Vec<Option<i64>> = vec![None; ictx.nb_streams() as usize];

    for (in_stream, mut packet) in ictx.packets() {
        let si = in_stream.index();
        let out_idx = match stream_mapping[si] {
            Some(i) => i,
            None => continue,
        };

        let pts = packet.pts().unwrap_or(0);

        // 跳过 end 之后的 packet（提前终止）
        if pts >= end_pts[si] {
            continue;
        }

        // 跳过 start 之前的非关键帧（保留关键帧以确保可解码）
        if pts < start_pts[si] {
            let medium = in_stream.parameters().medium();
            if medium == media::Type::Video && !packet.is_key() {
                continue;
            }
        }

        // 记录第一个 pts 作为偏移基准
        if first_pts[si].is_none() {
            first_pts[si] = Some(pts);
        }
        let offset = first_pts[si].unwrap_or(0);

        let in_tb = in_time_bases[si];
        let out_tb = out_time_bases[out_idx];

        packet.rescale_ts(in_tb, out_tb);

        // 偏移 pts/dts 让输出从 0 开始
        let offset_out = rescale_ts_value(offset, in_tb, out_tb);
        let new_pts = packet.pts().unwrap_or(0) - offset_out;
        let new_dts = packet.dts().unwrap_or(0) - offset_out;
        packet.set_pts(Some(new_pts.max(0)));
        packet.set_dts(Some(new_dts.max(0)));

        packet.set_position(-1);
        packet.set_stream(out_idx);
        packet.write_interleaved(&mut octx)?;
    }

    octx.write_trailer()?;

    let size = std::fs::metadata(output)?.len();
    Ok(size)
}

fn sec_to_pts(sec: f64, tb: Rational) -> i64 {
    (sec * f64::from(tb.denominator()) / f64::from(tb.numerator()).max(1.0)) as i64
}

fn rescale_ts_value(ts: i64, from: Rational, to: Rational) -> i64 {
    if from.numerator() == 0 || to.denominator() == 0 {
        return ts;
    }
    ts * i64::from(from.numerator()) * i64::from(to.denominator())
        / (i64::from(from.denominator()).max(1) * i64::from(to.numerator()).max(1))
}
