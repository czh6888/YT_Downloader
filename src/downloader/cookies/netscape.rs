/// Convert cookies to Netscape cookie file format.
/// Each cookie: (host, name, value, secure, httponly, expiry)
pub fn to_netscape(cookies: &[(String, String, String, bool, bool, String)]) -> String {
    let mut lines = vec!["# Netscape HTTP Cookie File".to_string(), String::new()];

    for (host, name, value, secure, _httponly, expiry) in cookies {
        // Skip empty values
        if value.is_empty() {
            continue;
        }
        let domain = if host.starts_with('.') {
            host.clone()
        } else {
            format!(".{host}")
        };
        lines.push(format!(
            "{}\tTRUE\t/\t{}\t{}\t{}\t{}",
            domain,
            if *secure { "TRUE" } else { "FALSE" },
            expiry,
            name,
            value,
        ));
    }

    lines.join("\n")
}
