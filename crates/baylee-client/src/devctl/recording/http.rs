//! Input bounds and CLI-only HTTP validation for local recording routes.

use std::io::BufRead;

/// Bound the entire HTTP head before allocation, including a single long line.
/// The parent applies this to all routes because routing follows head parsing.
pub(in crate::devctl) fn header_line(
    reader: &mut impl BufRead,
    remaining: &mut usize,
) -> Result<String, String> {
    let mut line = String::new();
    let read = std::io::Read::take(reader, *remaining as u64 + 1)
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    if read == 0 || read > *remaining || !line.ends_with('\n') {
        return Err("incomplete or oversized HTTP headers".into());
    }
    *remaining -= read;
    Ok(line)
}

/// Recording routes are CLI-only: reject browser requests and DNS rebinding,
/// and enforce verbs before putting anything on the app queue.
pub(in crate::devctl) fn validate_http(
    method: &str,
    path: &str,
    headers: &[(String, String)],
) -> Result<(), String> {
    if !path.starts_with("/recording/") {
        return Ok(());
    }
    let expected = match path {
        "/recording/status" => "GET",
        "/recording/start" | "/recording/stop" => "POST",
        _ => return Err("unknown recording route".into()),
    };
    if method != expected {
        return Err(format!("use {expected} for {path}"));
    }
    if headers
        .iter()
        .any(|(k, _)| k == "origin" || k.starts_with("sec-fetch-"))
    {
        return Err("recording is available only to local CLI clients".into());
    }
    let hosts: Vec<_> = headers.iter().filter(|(k, _)| k == "host").collect();
    if hosts.len() != 1 {
        return Err("one loopback Host header required".into());
    }
    let host = &hosts[0].1;
    let (name, port) = host
        .rsplit_once(':')
        .map_or((host.as_str(), None), |(h, p)| (h, Some(p)));
    if !matches!(name, "localhost" | "127.0.0.1")
        || port.is_some_and(|p| p.parse::<u16>().map_or(true, |n| n == 0))
    {
        return Err("loopback Host required".into());
    }
    Ok(())
}
