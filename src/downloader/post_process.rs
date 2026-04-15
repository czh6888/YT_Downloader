use std::process::Command;

/// 检测 ffmpeg 是否在 PATH 中。
pub fn detect_ffmpeg() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// 使用 ffmpeg 转换文件格式。
pub fn convert_file(input_path: &str, output_format: &str) -> Result<String, String> {
    if !detect_ffmpeg() {
        return Err("ffmpeg 未安装".to_string());
    }

    let output_path = format!(
        "{}.{}",
        input_path.rsplit('.').next().unwrap_or(input_path),
        output_format
    );

    let output = Command::new("ffmpeg")
        .args(["-i", input_path, "-c", "copy", "-y", &output_path])
        .output()
        .map_err(|e| format!("ffmpeg 启动失败: {e}"))?;

    if output.status.success() {
        Ok(output_path)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("ffmpeg 转换失败:\n{stderr}"))
    }
}
