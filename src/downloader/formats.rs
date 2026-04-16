/// Represents a downloadable format from yt-dlp output.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FormatInfo {
    pub format_id: String,
    pub ext: String,
    pub resolution: String,
    pub fps: Option<f64>,
    pub vcodec: String,
    pub acodec: String,
    pub filesize: Option<u64>,
    /// Approximate file size from yt-dlp (from Content-Length header).
    pub filesize_approx: Option<u64>,
    pub note: String,
    pub is_video: bool,
    pub is_audio: bool,
    pub is_combined: bool,
    pub height: Option<u32>,
    /// Best-effort total file size for progress bar.
    /// Priority: filesize > filesize_approx > (video + best_audio estimate).
    pub approx_total_size: Option<u64>,
}

/// Parse yt-dlp JSON output into a list of FormatInfo.
pub fn parse_formats(info: &serde_json::Value) -> Vec<FormatInfo> {
    let mut formats = Vec::new();

    let Some(fmts) = info.get("formats").and_then(|v| v.as_array()) else {
        return formats;
    };

    for fmt in fmts {
        let vcodec = fmt
            .get("vcodec")
            .and_then(|v| v.as_str())
            .unwrap_or("none")
            .to_string();
        let acodec = fmt
            .get("acodec")
            .and_then(|v| v.as_str())
            .unwrap_or("none")
            .to_string();

        let has_video = vcodec != "none";
        let has_audio = acodec != "none";

        if !has_video && !has_audio {
            continue;
        }

        let is_combined = has_video && has_audio;
        let is_video = has_video;
        let is_audio = has_audio && !has_video;

        let format_id = fmt
            .get("format_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let ext = fmt
            .get("ext")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let height = fmt.get("height").and_then(|v| v.as_u64()).map(|v| v as u32);
        let fps = fmt.get("fps").and_then(|v| v.as_f64());
        let filesize = fmt.get("filesize").and_then(|v| v.as_u64());
        let filesize_approx = fmt.get("filesize_approx").and_then(|v| v.as_u64());
        let note = fmt
            .get("format_note")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let resolution = if is_video {
            match height {
                Some(h) => format!("{h}p"),
                None => note.clone(),
            }
        } else {
            "audio only".to_string()
        };

        formats.push(FormatInfo {
            format_id,
            ext,
            resolution,
            fps,
            vcodec,
            acodec,
            filesize,
            filesize_approx,
            note,
            is_video,
            is_audio,
            is_combined,
            height,
            approx_total_size: None, // calculated below
        });
    }

    // Calculate approx_total_size for each format.
    // Strategy: each format already has its own filesize_approx from yt-dlp (Content-Length).
    // For video-only adaptive streams, estimate total = video filesize_approx + best audio filesize_approx.
    // For combined formats, use their own filesize or filesize_approx.

    // Find the best audio stream size for estimating combined downloads
    let best_audio_size: u64 = formats
        .iter()
        .filter(|f| f.is_audio && !f.is_combined)
        .filter_map(|f| f.filesize.or(f.filesize_approx))
        .max()
        .unwrap_or(0);

    for fmt in &mut formats {
        if fmt.is_combined {
            // Combined format: use its own filesize or filesize_approx
            fmt.approx_total_size = fmt.filesize.or(fmt.filesize_approx);
        } else if fmt.is_video {
            // Video-only adaptive stream:
            // 1. If it has filesize_approx, use it directly (this IS the video size)
            // 2. Estimate total = video size + best audio size
            if let Some(video_size) = fmt.filesize_approx {
                fmt.approx_total_size = Some(video_size + best_audio_size);
            } else if let Some(video_size) = fmt.filesize {
                fmt.approx_total_size = Some(video_size + best_audio_size);
            }
        }
        // Audio-only formats: use own filesize or filesize_approx
        if fmt.is_audio && fmt.approx_total_size.is_none() {
            fmt.approx_total_size = fmt.filesize.or(fmt.filesize_approx);
        }
    }

    // Sort: video by height descending (None → 0), audio by filesize descending (None → 0)
    formats.sort_by(|a, b| {
        if a.is_video && b.is_video {
            let a_h = a.height.unwrap_or(0);
            let b_h = b.height.unwrap_or(0);
            b_h.cmp(&a_h)
        } else if a.is_audio && b.is_audio {
            let a_f = a.filesize.unwrap_or(0);
            let b_f = b.filesize.unwrap_or(0);
            b_f.cmp(&a_f)
        } else if a.is_video {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    formats
}
