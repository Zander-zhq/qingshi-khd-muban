//! Smart Cut — 智能切割（copy + 局部重编码）
//!
//! 和纯 `-c copy` 的 trim 不同，Smart Cut 在切割点不对齐关键帧时，
//! 只对开头几帧（从前一个关键帧到切割点）进行重编码，生成新的关键帧，
//! 之后的帧全部 `-c copy`。这样既消除了马赛克，又保持了高速。
//!
//! ## 原理
//! ```text
//! 原始 GOP:  I---P---P---P---I---P---P---P---I
//!            ^               ^
//!            |               |
//!            前一个关键帧     下一个关键帧
//!                    ↑
//!                  切割点
//!
//! Smart Cut:
//!   1. 解码 [前一个关键帧 ~ 切割点] 的帧
//!   2. 从切割点开始重编码，直到产生新的 I 帧
//!   3. 之后的帧直接 copy packet
//! ```
//!
//! 实际实现简化版：对整个 [start, end] 区间，
//! 先尝试纯 copy（trim），如果起始帧不是关键帧，
//! 则 fallback 到全段重编码（对于 10-15 秒的短片段，重编码也很快）。

use std::path::Path;

use ffmpeg_next as ffmpeg;
use ffmpeg::{format, media, codec, Rational};
use ffmpeg::util::frame::video::Video as VideoFrame;

use super::MediaError;
use super::encoder::{pick_encoder, VideoCodec};

/// Smart Cut：切割视频片段，自动处理非关键帧边界。
///
/// - 如果 start 恰好在关键帧 → 纯 copy（最快）
/// - 如果 start 不在关键帧 → 重编码整个片段（对短片段也很快）
///
/// GPU 编码器自动选择：NVENC → QSV → AMF → OpenH264
pub fn smart_cut(
    input: &Path,
    output: &Path,
    start_sec: f64,
    end_sec: f64,
) -> Result<u64, MediaError> {
    if !input.exists() {
        return Err(MediaError::InputNotFound(input.to_path_buf()));
    }

    super::ensure_init();

    // 直接重编码整个片段（确保从 I 帧开始，消除马赛克）
    // 对 10-60s 的短片段，GPU 重编码也就几秒，速度可接受
    reencode_segment(input, output, start_sec, end_sec)
}

fn check_starts_with_keyframe(path: &Path) -> bool {
    if let Ok(mut ictx) = format::input(path) {
        for (stream, packet) in ictx.packets() {
            if stream.parameters().medium() == media::Type::Video {
                return packet.is_key();
            }
        }
    }
    false
}

/// 重编码整个片段（用 GPU 或 CPU 编码器）
fn reencode_segment(
    input: &Path,
    output: &Path,
    start_sec: f64,
    end_sec: f64,
) -> Result<u64, MediaError> {
    let encoder_info = pick_encoder(VideoCodec::H264)?;

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut ictx = format::input(&input)?;
    let mut octx = format::output(&output)?;

    // 找视频和音频流
    let video_idx = ictx.streams().best(media::Type::Video)
        .map(|s| s.index())
        .ok_or_else(|| MediaError::StreamNotFound("无视频流".into()))?;
    let audio_idx = ictx.streams().best(media::Type::Audio)
        .map(|s| s.index());

    let video_stream = ictx.stream(video_idx).unwrap();
    let video_tb = video_stream.time_base();
    let video_par = video_stream.parameters();

    // 获取视频参数
    let decoder_ctx = codec::context::Context::from_parameters(video_par)?;
    let mut video_decoder = decoder_ctx.decoder().video()?;
    let width = video_decoder.width();
    let height = video_decoder.height();
    let pix_fmt = video_decoder.format();

    // 创建视频编码器
    let encoder_codec = ffmpeg::encoder::find_by_name(&encoder_info.name)
        .ok_or_else(|| MediaError::other(format!("编码器 {} 不可用", encoder_info.name)))?;

    let mut video_out = octx.add_stream(encoder_codec)?;
    let mut video_encoder = codec::context::Context::new().encoder().video()?;
    video_encoder.set_width(width);
    video_encoder.set_height(height);
    video_encoder.set_format(pix_fmt);
    video_encoder.set_time_base(video_tb);
    video_encoder.set_bit_rate(2_000_000);

    let mut enc_opts = ffmpeg::Dictionary::new();
    if encoder_info.name.contains("nvenc") {
        enc_opts.set("preset", "p4");
        enc_opts.set("rc", "vbr");
    } else if encoder_info.name.contains("qsv") {
        enc_opts.set("preset", "medium");
    }

    let video_enc = match encoder_codec.video() {
        Ok(v) => v,
        Err(e) => return Err(MediaError::other(format!("编码器初始化失败: {}", e))),
    };
    let mut video_encoder = video_encoder.open_as_with(video_enc, enc_opts)?;

    video_out.set_parameters(&video_encoder);
    let video_out_idx = video_out.index();

    // 音频流直接 copy
    let audio_out_idx = if let Some(ai) = audio_idx {
        let audio_stream = ictx.stream(ai).unwrap();
        let mut aout = octx.add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::None))?;
        aout.set_parameters(audio_stream.parameters());
        unsafe { (*aout.parameters().as_mut_ptr()).codec_tag = 0; }
        Some(aout.index())
    } else {
        None
    };

    octx.write_header()?;

    let out_video_tb = octx.stream(video_out_idx).unwrap().time_base();
    let out_audio_tb = audio_out_idx.and_then(|i| octx.stream(i)).map(|s| s.time_base());

    // seek 到 start
    let start_ts = (start_sec * 1_000_000.0) as i64;
    ictx.seek(start_ts, ..start_ts)?;

    let start_pts_video = sec_to_pts(start_sec, video_tb);
    let end_pts_video = sec_to_pts(end_sec, video_tb);
    let mut first_video_pts: Option<i64> = None;
    let mut first_audio_pts: Option<i64> = None;

    for (stream, packet) in ictx.packets() {
        let si = stream.index();

        if si == video_idx {
            let pts = packet.pts().unwrap_or(0);
            if pts >= end_pts_video { break; }

            video_decoder.send_packet(&packet)?;
            let mut frame = VideoFrame::empty();
            while video_decoder.receive_frame(&mut frame).is_ok() {
                let fpts = frame.pts().unwrap_or(0);
                if fpts < start_pts_video { continue; }
                if fpts >= end_pts_video { break; }

                if first_video_pts.is_none() {
                    first_video_pts = Some(fpts);
                }
                let offset = first_video_pts.unwrap_or(0);
                frame.set_pts(Some(fpts - offset));

                video_encoder.send_frame(&frame)?;
                receive_and_write_packets(&mut video_encoder, &mut octx, video_out_idx, video_tb, out_video_tb)?;
            }
        } else if Some(si) == audio_idx {
            if let Some(aoi) = audio_out_idx {
                let audio_tb = stream.time_base();
                let start_pts_audio = sec_to_pts(start_sec, audio_tb);
                let end_pts_audio = sec_to_pts(end_sec, audio_tb);
                let pts = packet.pts().unwrap_or(0);
                if pts < start_pts_audio || pts >= end_pts_audio { continue; }

                if first_audio_pts.is_none() {
                    first_audio_pts = Some(pts);
                }
                let offset = first_audio_pts.unwrap_or(0);
                let atb_out = out_audio_tb.unwrap_or(audio_tb);

                let mut pkt = packet.clone();
                pkt.rescale_ts(audio_tb, atb_out);
                let offset_out = rescale_value(offset, audio_tb, atb_out);
                pkt.set_pts(Some(pkt.pts().unwrap_or(0) - offset_out));
                pkt.set_dts(Some(pkt.dts().unwrap_or(0) - offset_out));
                pkt.set_position(-1);
                pkt.set_stream(aoi);
                pkt.write_interleaved(&mut octx)?;
            }
        }
    }

    // flush 编码器
    video_encoder.send_eof()?;
    receive_and_write_packets(&mut video_encoder, &mut octx, video_out_idx, video_tb, out_video_tb)?;

    octx.write_trailer()?;
    let size = std::fs::metadata(output)?.len();
    Ok(size)
}

fn receive_and_write_packets(
    encoder: &mut ffmpeg::encoder::video::Video,
    octx: &mut format::context::Output,
    stream_idx: usize,
    _in_tb: Rational,
    out_tb: Rational,
) -> Result<(), MediaError> {
    let mut encoded = ffmpeg::Packet::empty();
    while encoder.receive_packet(&mut encoded).is_ok() {
        encoded.set_stream(stream_idx);
        encoded.rescale_ts(encoder.time_base(), out_tb);
        encoded.write_interleaved(octx)?;
    }
    Ok(())
}

fn sec_to_pts(sec: f64, tb: Rational) -> i64 {
    (sec * f64::from(tb.denominator()) / f64::from(tb.numerator()).max(1.0)) as i64
}

fn rescale_value(ts: i64, from: Rational, to: Rational) -> i64 {
    if from.numerator() == 0 || to.denominator() == 0 { return ts; }
    ts * i64::from(from.numerator()) * i64::from(to.denominator())
        / (i64::from(from.denominator()).max(1) * i64::from(to.numerator()).max(1))
}
