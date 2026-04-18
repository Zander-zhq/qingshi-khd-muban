//! 视频拼接（concat remux）
//!
//! 相当于 `ffmpeg -f concat -safe 0 -i filelist.txt -c copy output.mp4`：
//! 把多个编码一致的视频文件按顺序**复用**（不重新编解码）拼成一个文件。
//!
//! 典型用途：
//! - HLS/m3u8 下载后把 N 个 TS 分片合并成单个 MP4
//! - 录屏/录制的分段文件合并
//!
//! ## 使用约束
//! - 所有输入文件**必须使用相同的 codec / 分辨率 / 采样率 / 像素格式**，
//!   否则输出文件在播放器里会卡顿或画面异常
//! - TS 分片 + MP4 输出的组合，libavformat 会自动把 MPEG-TS 中的 AVC/AAC
//!   封装到 MP4 容器里，无需显式转码
//! - 时间戳会自动累加：第 N 个输入的 PTS 基于前 (N-1) 个文件时长偏移
//!
//! ## 不适用场景
//! - 输入文件的 codec/分辨率不一致 → 应该用 `merge` 模块（重编码后合并）
//! - 需要过滤/滤镜 → 用 `filter` 模块
//! - 需要裁剪某段时间 → 用 `trim` 模块

use std::path::{Path, PathBuf};

use ffmpeg_next as ffmpeg;
use ffmpeg::{format, media, Rational};

use super::MediaError;

/// 按顺序 remux 合并多个输入文件到一个输出（不重新编码）。
///
/// - `inputs`：输入文件路径列表，**必须至少一个**；文件 codec/参数要一致
/// - `output`：输出文件路径，容器格式由后缀自动推断（`.mp4` / `.mkv` / `.ts` 等）
///
/// 返回：输出文件的字节数（成功时从磁盘 metadata 获取）
///
/// ## 线程
/// 函数内部是同步阻塞的（libav 都是同步 C API）。调用方如果在 async 上下文，
/// 建议用 `tokio::task::spawn_blocking` 包一下，避免阻塞 runtime。
pub fn concat_remux(inputs: &[PathBuf], output: &Path) -> Result<u64, MediaError> {
    if inputs.is_empty() {
        return Err(MediaError::EmptyInput);
    }
    // 预检查所有文件都存在
    for f in inputs {
        if !f.exists() {
            return Err(MediaError::InputNotFound(f.clone()));
        }
    }

    super::ensure_init();

    // 确保输出目录存在
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // ---- 1. 用第一个输入作为"流模板"，建输出容器 ----
    let first_ictx = format::input(&inputs[0])?;
    let mut octx = format::output(&output)?;

    // 记录"输入 stream index → 输出 stream index"的映射表。
    // 只复制 video + audio 流（字幕/数据流等先忽略，足够 HLS TS 合并场景）。
    let mut stream_mapping: Vec<Option<usize>> = vec![None; first_ictx.nb_streams() as usize];
    let mut out_time_bases: Vec<Rational> = Vec::new();
    let mut next_out_idx = 0usize;

    for in_stream in first_ictx.streams() {
        let medium = in_stream.parameters().medium();
        if medium != media::Type::Video && medium != media::Type::Audio {
            continue;
        }

        // 新建输出流并复制编码参数
        let mut out_stream = octx.add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::None))?;
        out_stream.set_parameters(in_stream.parameters());

        // 清掉 codec_tag，让 muxer 根据容器自动选合适的 tag
        // （MPEG-TS 里的 tag 可能和 MP4 不兼容，不清会 muxer 报错）
        unsafe {
            (*out_stream.parameters().as_mut_ptr()).codec_tag = 0;
        }

        stream_mapping[in_stream.index()] = Some(next_out_idx);
        out_time_bases.push(Rational::new(0, 1)); // 占位，write_header 后重新取真实 time_base
        next_out_idx += 1;
    }
    drop(first_ictx);

    if next_out_idx == 0 {
        return Err(MediaError::StreamNotFound("输入文件没有视频或音频流".into()));
    }

    // 写输出头。这之后输出流的 time_base 由 muxer 决定（MP4 一般是 1/90000 或 1/采样率）
    octx.write_header()?;

    // 更新每个输出流真实的 time_base
    for (idx, tb_slot) in out_time_bases.iter_mut().enumerate() {
        let os = octx.stream(idx as usize).ok_or_else(|| {
            MediaError::other(format!("output stream[{}] 获取失败", idx))
        })?;
        *tb_slot = os.time_base();
    }

    // ---- 2. 按顺序读每个输入文件，rescale 时间戳写到输出 ----
    // pts_offset[out_stream_idx] 记录每个输出流当前的"时间戳偏移"。
    // 每切到下一个输入文件时，用该文件的累计时长（按输出流 time_base）累加。
    let mut pts_offset: Vec<i64> = vec![0; next_out_idx];
    // 每个输出流在当前文件结束时最后一个 packet 的 pts + duration
    // （用来算下一个文件的起始偏移）
    let mut file_last_end: Vec<i64> = vec![0; next_out_idx];

    for (file_idx, input_path) in inputs.iter().enumerate() {
        let mut ictx = format::input(&input_path)?;

        // 建立本次输入的流映射（以"输入 stream index → 输出 stream index"）
        // 第一个文件的映射已经在上面建好了；后续文件要重新根据当前 ictx 建映射，
        // 保持"按流类型顺序对应"的策略：视频对视频、第一个音轨对第一个音轨，以此类推。
        let mut cur_map: Vec<Option<usize>> = vec![None; ictx.nb_streams() as usize];
        {
            let mut assigned_video = Vec::<usize>::new();
            let mut assigned_audio = Vec::<usize>::new();
            // 先收集输出流里每个 kind 有几个
            let mut out_video_idx: Vec<usize> = Vec::new();
            let mut out_audio_idx: Vec<usize> = Vec::new();
            for i in 0..next_out_idx {
                if let Some(os) = octx.stream(i) {
                    match os.parameters().medium() {
                        media::Type::Video => out_video_idx.push(i),
                        media::Type::Audio => out_audio_idx.push(i),
                        _ => {}
                    }
                }
            }

            for in_s in ictx.streams() {
                let kind = in_s.parameters().medium();
                match kind {
                    media::Type::Video => {
                        let n = assigned_video.len();
                        if let Some(&o) = out_video_idx.get(n) {
                            cur_map[in_s.index()] = Some(o);
                            assigned_video.push(o);
                        }
                    }
                    media::Type::Audio => {
                        let n = assigned_audio.len();
                        if let Some(&o) = out_audio_idx.get(n) {
                            cur_map[in_s.index()] = Some(o);
                            assigned_audio.push(o);
                        }
                    }
                    _ => {}
                }
            }
        }

        // 非首文件：把偏移推进到上个文件结束位置
        if file_idx > 0 {
            for i in 0..next_out_idx {
                pts_offset[i] += file_last_end[i];
                file_last_end[i] = 0;
            }
        }

        for (in_stream, mut packet) in ictx.packets() {
            let out_idx = match cur_map[in_stream.index()] {
                Some(i) => i,
                None => continue, // 不映射的流（字幕等）跳过
            };

            let in_tb = in_stream.time_base();
            let out_tb = out_time_bases[out_idx];

            // 按输出流 time_base 重缩放 pts/dts/duration
            packet.rescale_ts(in_tb, out_tb);

            // 累加偏移
            let pts = packet.pts().unwrap_or(0) + pts_offset[out_idx];
            let dts = packet.dts().unwrap_or(0) + pts_offset[out_idx];
            packet.set_pts(Some(pts));
            packet.set_dts(Some(dts));

            // 维护当前输出流的"当前文件 last end"：pts + duration
            let dur = packet.duration();
            let cur_end = pts - pts_offset[out_idx] + dur;
            if cur_end > file_last_end[out_idx] {
                file_last_end[out_idx] = cur_end;
            }

            packet.set_position(-1);
            packet.set_stream(out_idx);
            packet.write_interleaved(&mut octx)?;
        }
    }

    // ---- 3. 收尾 ----
    octx.write_trailer()?;

    let size = std::fs::metadata(output)?.len();
    Ok(size)
}
