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

use std::path::Path;

use ffmpeg_next as ffmpeg;
use ffmpeg::{format, media, Rational};

use super::MediaError;

/// 将单个输入文件转封装到指定输出格式（不重新编码）。
///
/// - `input`：输入文件路径（.ts / .flv / .mkv 等）
/// - `output`：输出文件路径，容器格式由后缀推断（.mp4 / .mkv 等）
///
/// 返回：输出文件的字节数
pub fn remux(input: &Path, output: &Path) -> Result<u64, MediaError> {
    if !input.exists() {
        return Err(MediaError::InputNotFound(input.to_path_buf()));
    }

    super::ensure_init();

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut ictx = format::input(&input)?;
    let mut octx = format::output(&output)?;

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

    for (in_stream, mut packet) in ictx.packets() {
        let out_idx = match stream_mapping[in_stream.index()] {
            Some(i) => i,
            None => continue,
        };

        let in_tb = in_stream.time_base();
        let out_tb = out_time_bases[out_idx];

        packet.rescale_ts(in_tb, out_tb);
        packet.set_position(-1);
        packet.set_stream(out_idx);
        packet.write_interleaved(&mut octx)?;
    }

    octx.write_trailer()?;

    let size = std::fs::metadata(output)?.len();
    Ok(size)
}
