use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

use super::netscape;

/// Extract cookies from Firefox's cookies.sqlite.
/// Firefox cookies are NOT encrypted (unlike Chromium).
pub fn extract_cookies(cookie_file: &PathBuf) -> Result<()> {
    let profile_dir = firefox_profile_dir()?;
    let db_path = profile_dir.join("cookies.sqlite");

    if !db_path.exists() {
        anyhow::bail!(
            "Firefox cookie database not found:\n{}\n\nPlease open Firefox and log in first.",
            db_path.display()
        );
    }

    // Copy to temp to avoid locking
    let tmp_dir = tempfile::tempdir()?;
    let tmp_db = tmp_dir.path().join("cookies.sqlite");
    std::fs::copy(&db_path, &tmp_db).context("Failed to copy Firefox cookie database")?;

    let conn = Connection::open(&tmp_db)?;

    let mut stmt = conn.prepare(
        "SELECT host, name, value, isSecure, isHttpOnly, expiry FROM moz_cookies",
    )?;

    let cookies: Vec<(String, String, String, bool, bool, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, bool>(4)?,
                row.get::<_, Option<i64>>(5)?.unwrap_or(0).to_string(),
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let content = netscape::to_netscape(&cookies);
    std::fs::write(cookie_file, content)
        .with_context(|| format!("Failed to write cookie file to {}", cookie_file.display()))?;

    Ok(())
}

fn firefox_profile_dir() -> Result<PathBuf> {
    let up = std::env::var("USERPROFILE")?;
    let profiles_dir = PathBuf::from(&up)
        .join("AppData")
        .join("Roaming")
        .join("Mozilla")
        .join("Firefox")
        .join("Profiles");

    if !profiles_dir.exists() {
        anyhow::bail!("Firefox profiles not found. Please make sure Firefox is installed.");
    }

    for entry in std::fs::read_dir(&profiles_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
                && (name.ends_with(".default-release") || name.ends_with(".default")) {
                    return Ok(path);
                }
    }

    anyhow::bail!("No Firefox default profile found. Please open Firefox and log in first.");
}
