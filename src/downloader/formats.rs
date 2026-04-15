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
    pub note: String,
    pub is_video: bool,
    pub is_audio: bool,
    pub height: Option<u32>,
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

        // For sorting: if both, treat as video primary
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
            note,
            is_video,
            is_audio,
            height,
        });
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
