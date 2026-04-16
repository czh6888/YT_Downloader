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

static PCT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\[download\].*?([\d.]+)%").unwrap()
});

static SIZE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"of\s+[~\s]*([\d.]+)\s*([KMGT]?i?B)").unwrap()
});

static SPEED_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"at\s+([\d.]+)\s*([KMGT]?i?B/s)").unwrap()
});

static ETA_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"ETA\s+(\d{1,2}):(\d{2})").unwrap()
});

/// Try to parse a yt-dlp output line as a progress update.
pub fn parse_progress(line: &str) -> Option<ProgressInfo> {
    // Must start with [download] and contain a percentage
    if !line.contains("[download]") {
        return None;
    }

    let percentage = PCT_RE.captures(line)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f64>().ok())?;

    let speed = SPEED_RE.captures(line)
        .and_then(|c| {
            let val = c.get(1)?.as_str().parse::<f64>().ok()?;
            let unit = c.get(2)?.as_str();
            Some(val * parse_size_unit(unit))
        });

    let eta = ETA_RE.captures(line)
        .and_then(|c| {
            let mins = c.get(1)?.as_str().parse::<u64>().ok()?;
            let secs = c.get(2)?.as_str().parse::<u64>().ok()?;
            Some(mins * 60 + secs)
        });

    let (downloaded, total) = SIZE_RE.captures(line)
        .and_then(|c| {
            let val = c.get(1)?.as_str().parse::<f64>().ok()?;
            let unit = c.get(2)?.as_str();
            let bytes = (val * parse_size_unit(unit)) as u64;
            let total = if percentage > 0.0 {
                Some((bytes as f64 / percentage * 100.0) as u64)
            } else {
                None
            };
            Some((Some(bytes), total))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_progress_full() {
        let line = "[download]  45.2% of ~  12.34MiB at    2.15MiB/s ETA 00:05";
        let p = parse_progress(line).unwrap();
        assert!((p.percentage - 45.2).abs() < 0.01);
        assert!(p.speed.is_some());
        assert!(p.eta.is_some());
        assert!(p.downloaded.is_some());
    }

    #[test]
    fn test_parse_progress_pct_only() {
        let line = "[download]  75.0%";
        let p = parse_progress(line).unwrap();
        assert!((p.percentage - 75.0).abs() < 0.01);
        assert!(p.speed.is_none());
        assert!(p.eta.is_none());
    }

    #[test]
    fn test_parse_progress_with_eta() {
        let line = "[download] 100% of 50.00MiB in 00:02";
        let p = parse_progress(line).unwrap();
        assert!((p.percentage - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_non_progress_line() {
        let line = "[info] Video title here";
        assert!(parse_progress(line).is_none());
    }
}
