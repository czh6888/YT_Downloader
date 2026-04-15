use regex::Regex;
use std::sync::LazyLock;

/// Parsed download progress from yt-dlp output.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProgressInfo {
    pub percentage: f64,
    pub speed: Option<f64>,       // bytes/sec
    pub eta: Option<u64>,         // seconds
    pub downloaded: Option<u64>,  // bytes
    pub total: Option<u64>,       // bytes
}

// Matches lines like:
// [download]  45.2% of ~  12.34MiB at    2.15MiB/s ETA 00:05
// [download] 100% of  123.45MiB in 00:01:23
static PROGRESS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\[download\]\s+([\d.]+)%.*?(?:of\s+[~\s]*([\d.]+)([KMGT]?i?B))?\s*(?:at\s+([\d.]+)([KMGT]?i?B/s))?\s*(?:ETA\s+(\d{2}:\d{2}))?",
    )
    .unwrap()
});

/// Try to parse a yt-dlp output line as a progress update.
pub fn parse_progress(line: &str) -> Option<ProgressInfo> {
    let caps = PROGRESS_RE.captures(line)?;

    let percentage = caps.get(1)?.as_str().parse::<f64>().ok()?;

    let speed = caps.get(4).and_then(|m| m.as_str().parse::<f64>().ok());
    let eta = caps.get(6).and_then(|m| parse_eta(m.as_str()));

    let (downloaded, total) = caps
        .get(2)
        .and_then(|m| m.as_str().parse::<f64>().ok())
        .map(|val| {
            let multiplier = caps
                .get(3)
                .map(|u| parse_size_unit(u.as_str()))
                .unwrap_or(1.0);
            let bytes = (val * multiplier) as u64;
            let total = if percentage > 0.0 {
                Some((bytes as f64 / percentage * 100.0) as u64)
            } else {
                None
            };
            (Some(bytes), total)
        })
        .unwrap_or((None, None));

    Some(ProgressInfo {
        percentage,
        speed,
        eta,
        downloaded,
        total,
    })
}

fn parse_eta(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 2 {
        let mins = parts[0].parse::<u64>().ok()?;
        let secs = parts[1].parse::<u64>().ok()?;
        Some(mins * 60 + secs)
    } else {
        None
    }
}

fn parse_size_unit(unit: &str) -> f64 {
    match unit {
        "B" => 1.0,
        "KiB" | "KB" => 1024.0,
        "MiB" | "MB" => 1024.0 * 1024.0,
        "GiB" | "GB" => 1024.0 * 1024.0 * 1024.0,
        "TiB" | "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    }
}
