//! 音频静音检测（silence detection）
//!
//! 扫描输入文件的音频轨，找出所有"静音段"的起止时间。
//! 只解码音频帧（不处理视频），速度很快。
//!
//! 典型用途：
//! - 智能分割视频时，优先在静音点切割（避免切断语音）
//! - 检测视频是否有音频、音频是否全静音
//!
//! ## 算法
//! 1. 打开输入文件，找到第一条音频流
//! 2. 逐帧解码音频，计算每帧的 RMS（均方根）能量
//! 3. RMS 低于阈值的连续帧标记为"静音中"
//! 4. 静音持续时间超过 `min_duration` 的记录为一个静音段

use std::path::Path;

use ffmpeg_next as ffmpeg;
use ffmpeg::{format, media, codec, util::frame::audio::Audio};

use super::MediaError;

/// 一个静音段的起止时间（秒）
#[derive(Debug, Clone, serde::Serialize)]
pub struct SilenceRange {
    pub start_sec: f64,
    pub end_sec: f64,
}

impl SilenceRange {
    /// 静音段的中点时间
    pub fn mid(&self) -> f64 {
        (self.start_sec + self.end_sec) / 2.0
    }
}

/// 检测输入文件音频轨中的所有静音段。
///
/// - `input`：输入文件路径
/// - `threshold_db`：静音阈值（分贝），如 -30.0。低于此值视为静音。
/// - `min_duration_sec`：最短静音持续时长（秒），如 0.3。短于此值的静音忽略。
///
/// 返回：按时间排序的静音段列表
pub fn detect_silence(
    input: &Path,
    threshold_db: f64,
    min_duration_sec: f64,
) -> Result<Vec<SilenceRange>, MediaError> {
    if !input.exists() {
        return Err(MediaError::InputNotFound(input.to_path_buf()));
    }

    super::ensure_init();

    let mut ictx = format::input(&input)?;

    // 找第一条音频流
    let audio_stream_idx = ictx.streams()
        .best(media::Type::Audio)
        .map(|s| s.index())
        .ok_or_else(|| MediaError::StreamNotFound("输入文件没有音频流".into()))?;

    let audio_stream = ictx.stream(audio_stream_idx).unwrap();
    let audio_tb = audio_stream.time_base();
    let codec_par = audio_stream.parameters();

    // 创建音频解码器
    let context = codec::context::Context::from_parameters(codec_par)?;
    let mut decoder = context.decoder().audio()?;

    let threshold_linear = db_to_linear(threshold_db);
    let mut silences: Vec<SilenceRange> = Vec::new();
    let mut in_silence = false;
    let mut silence_start: f64 = 0.0;

    for (stream, packet) in ictx.packets() {
        if stream.index() != audio_stream_idx {
            continue;
        }

        decoder.send_packet(&packet)?;

        let mut frame = Audio::empty();
        while decoder.receive_frame(&mut frame).is_ok() {
            let rms = compute_rms(&frame);
            let pts = frame.pts().unwrap_or(0);
            let time_sec = pts as f64 * f64::from(audio_tb.numerator())
                / f64::from(audio_tb.denominator()).max(1.0);

            if rms < threshold_linear {
                if !in_silence {
                    in_silence = true;
                    silence_start = time_sec;
                }
            } else {
                if in_silence {
                    let duration = time_sec - silence_start;
                    if duration >= min_duration_sec {
                        silences.push(SilenceRange {
                            start_sec: silence_start,
                            end_sec: time_sec,
                        });
                    }
                    in_silence = false;
                }
            }
        }
    }

    // 处理文件末尾的静音段
    if in_silence {
        let total_duration = ictx.duration() as f64 / 1_000_000.0;
        let duration = total_duration - silence_start;
        if duration >= min_duration_sec {
            silences.push(SilenceRange {
                start_sec: silence_start,
                end_sec: total_duration,
            });
        }
    }

    Ok(silences)
}

/// 计算音频帧的 RMS（均方根）能量，结果为线性幅度 [0.0, 1.0]
fn compute_rms(frame: &Audio) -> f64 {
    let samples = frame.samples();
    let channels = frame.channels() as usize;
    if samples == 0 || channels == 0 {
        return 0.0;
    }

    let fmt = frame.format();
    let total_samples = samples * channels;

    let sum_sq: f64 = match fmt {
        format::Sample::I16(_) => {
            let data = frame.data(0);
            let slice = unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const i16, total_samples.min(data.len() / 2))
            };
            slice.iter().map(|&s| {
                let f = s as f64 / i16::MAX as f64;
                f * f
            }).sum()
        }
        format::Sample::I32(_) => {
            let data = frame.data(0);
            let slice = unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const i32, total_samples.min(data.len() / 4))
            };
            slice.iter().map(|&s| {
                let f = s as f64 / i32::MAX as f64;
                f * f
            }).sum()
        }
        format::Sample::F32(_) => {
            let data = frame.data(0);
            let slice = unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const f32, total_samples.min(data.len() / 4))
            };
            slice.iter().map(|&s| (s as f64) * (s as f64)).sum()
        }
        format::Sample::F64(_) => {
            let data = frame.data(0);
            let slice = unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const f64, total_samples.min(data.len() / 8))
            };
            slice.iter().map(|&s| s * s).sum()
        }
        _ => {
            // planar 格式：逐平面读取
            let mut sum = 0.0f64;
            let mut count = 0usize;
            for ch in 0..channels {
                let data = frame.data(ch);
                if data.is_empty() { continue; }
                // 尝试作为 f32 planar
                let plane_samples = samples.min(data.len() / 4);
                let slice = unsafe {
                    std::slice::from_raw_parts(data.as_ptr() as *const f32, plane_samples)
                };
                for &s in slice {
                    sum += (s as f64) * (s as f64);
                    count += 1;
                }
            }
            if count > 0 { return (sum / count as f64).sqrt(); }
            return 0.0;
        }
    };

    (sum_sq / total_samples as f64).sqrt()
}

fn db_to_linear(db: f64) -> f64 {
    10.0f64.powf(db / 20.0)
}
