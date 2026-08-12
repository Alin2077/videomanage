use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 探测到的视频元数据
#[derive(Debug, Clone, Default)]
pub struct VideoMeta {
    pub duration: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub codec: Option<String>,
    pub fps: Option<f64>,
    pub sample_rate: Option<i64>,
}

/// 查找 ffprobe 可执行文件：优先设置中配置的路径，其次 PATH
pub fn find_ffprobe(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = configured {
        if !p.trim().is_empty() {
            let path = PathBuf::from(p.trim());
            if path.is_file() {
                return Some(path);
            }
        }
    }
    // 尝试 PATH 中的 ffprobe / ffprobe.exe
    for name in ["ffprobe", "ffprobe.exe"] {
        if let Ok(p) = which(name) {
            return Some(p);
        }
    }
    None
}

/// 查找 ffmpeg 可执行文件
pub fn find_ffmpeg(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = configured {
        if !p.trim().is_empty() {
            let path = PathBuf::from(p.trim());
            if path.is_file() {
                return Some(path);
            }
        }
    }
    for name in ["ffmpeg", "ffmpeg.exe"] {
        if let Ok(p) = which(name) {
            return Some(p);
        }
    }
    None
}

fn which(name: &str) -> Result<PathBuf, String> {
    let path_var = std::env::var("PATH").map_err(|_| "无 PATH".to_string())?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!("未找到 {name}"))
}

/// 解析 "30000/1001" 形式的帧率字符串
fn parse_frame_rate(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some((num, den)) = s.split_once('/') {
        let n: f64 = num.trim().parse().ok()?;
        let d: f64 = den.trim().parse().ok()?;
        if d != 0.0 && n > 0.0 {
            return Some(n / d);
        }
        None
    } else {
        s.parse().ok()
    }
}

/// 使用 ffprobe 提取视频元数据
pub fn probe_video(ffprobe: &Path, file_path: &str) -> Result<VideoMeta, String> {
    let output = Command::new(ffprobe)
        .arg("-v")
        .arg("error")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(file_path)
        .output()
        .map_err(|e| format!("运行 ffprobe 失败: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("ffprobe 错误: {err}"));
    }

    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("解析 ffprobe 输出失败: {e}"))?;

    let mut meta = VideoMeta::default();

    if let Some(dur) = json
        .pointer("/format/duration")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
    {
        meta.duration = Some(dur);
    }

    if let Some(streams) = json.get("streams").and_then(|v| v.as_array()) {
        let mut video_stream = None;
        let mut audio_stream = None;
        for s in streams {
            let codec_type = s.get("codec_type").and_then(|v| v.as_str()).unwrap_or("");
            match codec_type {
                "video" if video_stream.is_none() => video_stream = Some(s),
                "audio" if audio_stream.is_none() => audio_stream = Some(s),
                _ => {}
            }
        }
        if let Some(vs) = video_stream {
            meta.width = vs.get("width").and_then(|v| v.as_i64());
            meta.height = vs.get("height").and_then(|v| v.as_i64());
            meta.codec = vs.get("codec_name").and_then(|v| v.as_str()).map(String::from);
            meta.fps = vs
                .get("r_frame_rate")
                .and_then(|v| v.as_str())
                .and_then(parse_frame_rate);
        }
        if let Some(aus) = audio_stream {
            meta.sample_rate = aus
                .get("sample_rate")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok());
        }
    }

    Ok(meta)
}

/// 使用 ffmpeg 截取视频 10% 处帧作为封面，返回封面文件路径
pub fn generate_cover(
    ffmpeg: &Path,
    file_path: &str,
    duration: Option<f64>,
    cover_out_dir: &Path,
    video_id: i64,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(cover_out_dir).map_err(|e| format!("创建封面目录失败: {e}"))?;
    let cover_path = cover_out_dir.join(format!("cover_{video_id}.jpg"));

    // 计算截取时间点：取 10% 处，避免片头黑屏；限制在 1~60 秒之间
    let seek = match duration {
        Some(d) if d.is_finite() && d > 5.0 => (d * 0.1).clamp(1.0, 60.0),
        _ => 1.0,
    };

    let output = Command::new(ffmpeg)
        .arg("-y")
        .arg("-ss")
        .arg(format!("{seek:.2}"))
        .arg("-i")
        .arg(file_path)
        .arg("-vframes")
        .arg("1")
        .arg("-vf")
        .arg("scale=320:-2")
        .arg("-q:v")
        .arg("4")
        .arg(&cover_path)
        .output()
        .map_err(|e| format!("运行 ffmpeg 失败: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("封面生成失败: {err}"));
    }
    if cover_path.is_file() {
        Ok(cover_path)
    } else {
        Err("封面文件未生成".to_string())
    }
}

/// 计算文件 SHA-256（用于去重）
pub fn file_sha256(path: &str) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| format!("打开文件失败: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("读取文件失败: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}
