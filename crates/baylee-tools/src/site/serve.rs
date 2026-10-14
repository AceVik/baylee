//! Loopback-only preview of public web files, with no listings or request logs.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};

const MAX_HEADER: usize = 8192;
const TIMEOUT: Duration = Duration::from_secs(5);

pub(super) fn address(bind: &str, port: u16) -> Result<SocketAddr> {
    let bind: IpAddr = bind
        .parse()
        .context("preview bind must be a loopback IP address")?;
    ensure!(
        bind.is_loopback(),
        "preview server may bind only to loopback"
    );
    Ok(SocketAddr::new(bind, port))
}

fn private(path: &Path) -> bool {
    path.components().any(|c| match c {
        Component::Normal(n) => {
            let n = n.to_string_lossy();
            n.starts_with('.')
                || n == "private"
                || n == "node_modules"
                || n.ends_with(".bin")
                || (n.starts_with("impressum-data") && n.ends_with(".json"))
        }
        Component::ParentDir => true,
        _ => false,
    })
}

pub(super) fn file(web: &Path, target: &str) -> Option<PathBuf> {
    if !target.starts_with('/') || target.starts_with("//") {
        return None;
    }
    let target = target.split(['?', '#']).next()?;
    let mut bytes = Vec::with_capacity(target.len());
    let mut input = target.bytes();
    while let Some(b) = input.next() {
        if b == b'%' {
            let a = char::from(input.next()?).to_digit(16)?;
            let b = char::from(input.next()?).to_digit(16)?;
            bytes.push((a * 16 + b) as u8);
        } else {
            bytes.push(b);
        }
    }
    let path = std::str::from_utf8(&bytes).ok()?;
    if path.contains(['\0', '\\']) {
        return None;
    }
    let relative = Path::new(path.strip_prefix('/')?);
    if relative.is_absolute() || private(relative) {
        return None;
    }
    let web = web.canonicalize().ok()?;
    let mut path = web.join(relative).canonicalize().ok()?;
    if path.is_dir() {
        path = path.join("index.html").canonicalize().ok()?;
    }
    let relative = path.strip_prefix(&web).ok()?;
    if private(relative) || !path.is_file() {
        return None;
    }
    Some(path)
}

pub(super) fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "txt" => "text/plain; charset=utf-8",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "webm" => "video/webm",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

fn respond(
    stream: &mut impl Write,
    status: &str,
    body: &[u8],
    head: bool,
    allow: bool,
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n",
        body.len()
    )?;
    if allow {
        write!(stream, "Allow: GET, HEAD\r\n")?;
    }
    write!(stream, "\r\n")?;
    if !head {
        stream.write_all(body)?;
    }
    Ok(())
}

pub(super) fn connection(web: &Path, mut stream: TcpStream) -> Result<()> {
    stream.set_write_timeout(Some(TIMEOUT))?;
    let start = Instant::now();
    let mut header = Vec::new();
    loop {
        let Some(remaining) = TIMEOUT.checked_sub(start.elapsed()) else {
            respond(
                &mut stream,
                "408 Request Timeout",
                b"request timeout\n",
                false,
                false,
            )?;
            return Ok(());
        };
        stream.set_read_timeout(Some(remaining))?;
        let mut buffer = [0; 1024];
        let size = match stream.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(n) => n,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                respond(
                    &mut stream,
                    "408 Request Timeout",
                    b"request timeout\n",
                    false,
                    false,
                )?;
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };
        header.extend_from_slice(&buffer[..size]);
        if let Some(end) = header.windows(4).position(|w| w == b"\r\n\r\n") {
            if end + 4 > MAX_HEADER {
                respond(
                    &mut stream,
                    "431 Request Header Fields Too Large",
                    b"headers too large\n",
                    false,
                    false,
                )?;
                return Ok(());
            }
            header.truncate(end + 4);
            break;
        }
        if header.len() >= MAX_HEADER {
            respond(
                &mut stream,
                "431 Request Header Fields Too Large",
                b"headers too large\n",
                false,
                false,
            )?;
            return Ok(());
        }
    }
    reply(web, &header, &mut stream)
}

fn reply(web: &Path, header: &[u8], stream: &mut impl Write) -> Result<()> {
    let end = header.windows(4).position(|w| w == b"\r\n\r\n");
    if end.is_none_or(|end| end + 4 > MAX_HEADER) {
        respond(
            stream,
            "431 Request Header Fields Too Large",
            b"headers too large\n",
            false,
            false,
        )?;
        return Ok(());
    }
    let text = std::str::from_utf8(header).unwrap_or("");
    let mut request = text.lines().next().unwrap_or("").split_whitespace();
    let method = request.next().unwrap_or("");
    let target = request.next().unwrap_or("");
    let version = request.next().unwrap_or("");
    let head = method == "HEAD";
    if request.next().is_some() || !["HTTP/1.0", "HTTP/1.1"].contains(&version) {
        respond(stream, "400 Bad Request", b"bad request\n", head, false)?;
        return Ok(());
    }
    if !["GET", "HEAD"].contains(&method) {
        respond(
            stream,
            "405 Method Not Allowed",
            b"method not allowed\n",
            false,
            true,
        )?;
        return Ok(());
    }
    let Some(path) = file(web, target) else {
        respond(stream, "404 Not Found", b"not found\n", head, false)?;
        return Ok(());
    };
    let Ok(file) = File::open(&path) else {
        respond(stream, "404 Not Found", b"not found\n", head, false)?;
        return Ok(());
    };
    let size = file.metadata()?.len();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {size}\r\nCache-Control: no-cache\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        mime(&path)
    )?;
    if !head {
        std::io::copy(&mut file.take(size), stream)?;
    }
    Ok(())
}

pub(super) fn run(root: &Path, port: u16, bind: &str) -> Result<()> {
    let address = address(bind, port)?;
    let web = fs::canonicalize(root.join("web"))?;
    let listener = TcpListener::bind(address)?;
    println!("preview: http://{}/site/", listener.local_addr()?);
    // Preview requests and OS errors may contain private paths: never log them.
    for stream in listener.incoming().flatten() {
        let _ = connection(&web, stream);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join("site/data")).unwrap();
        fs::write(d.path().join("site/index.html"), "<p>preview</p>").unwrap();
        fs::write(d.path().join("site/data/test.json"), "{}").unwrap();
        d
    }
    #[test]
    fn preview_binds_only_to_loopback() {
        for bind in ["127.0.0.1", "::1", "127.0.0.2"] {
            assert!(address(bind, 8000).is_ok());
        }
        for bind in ["0.0.0.0", "::", "192.168.1.1", "host.invalid"] {
            assert!(address(bind, 8000).is_err());
        }
    }
    #[test]
    fn preview_preserves_site_route_and_refuses_listing_private_and_traversal_paths() {
        let d = fixture();
        assert!(
            file(d.path(), "/site/")
                .unwrap()
                .ends_with("site/index.html")
        );
        assert!(file(d.path(), "/site/data/test.json?v=a").is_some());
        for target in [
            "/site/data/",
            "/../secret",
            "/%2e%2e/secret",
            "/site/%2e%2e/%2e%2e/secret",
            "//etc/passwd",
            "http://host.invalid/site/",
            "/%00",
            "/%GG",
            "/%",
            "/%ff",
            "/site\\index.html",
        ] {
            assert!(file(d.path(), target).is_none(), "{target}");
        }
        fs::create_dir(d.path().join("site/private")).unwrap();
        for name in [
            "site/private/anything.txt",
            "site/.env",
            "site/impressum-data-2.json",
            "site/blob.bin",
        ] {
            fs::write(d.path().join(name), "secret").unwrap();
            assert!(file(d.path(), &format!("/{name}")).is_none());
        }
    }
    #[cfg(unix)]
    #[test]
    fn preview_fences_symlinks_to_outside_or_private_paths() {
        use std::os::unix::fs::symlink;
        let d = fixture();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("x.txt"), "secret").unwrap();
        symlink(outside.path().join("x.txt"), d.path().join("site/link.txt")).unwrap();
        assert!(file(d.path(), "/site/link.txt").is_none());
        fs::create_dir(d.path().join("private")).unwrap();
        fs::write(d.path().join("private/x.txt"), "secret").unwrap();
        symlink(
            d.path().join("private/x.txt"),
            d.path().join("site/secret.txt"),
        )
        .unwrap();
        assert!(file(d.path(), "/site/secret.txt").is_none());
    }
    fn request(web: &Path, request: &[u8]) -> String {
        let mut bytes = Vec::new();
        reply(web, request, &mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }
    #[test]
    fn preview_get_head_methods_headers_mime_and_no_cache() {
        let d = fixture();
        let get = request(d.path(), b"GET /site/ HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert!(get.starts_with("HTTP/1.1 200"));
        assert!(get.ends_with("<p>preview</p>"));
        assert!(get.contains("Cache-Control: no-cache"));
        assert!(get.contains("Content-Type: text/html; charset=utf-8"));
        let head = request(d.path(), b"HEAD /site/ HTTP/1.1\r\n\r\n");
        assert!(head.starts_with("HTTP/1.1 200"));
        assert!(head.ends_with("\r\n\r\n"));
        assert!(head.contains("Content-Length: 14"));
        let post = request(d.path(), b"POST /site/ HTTP/1.1\r\n\r\n");
        assert!(post.starts_with("HTTP/1.1 405"));
        assert!(post.contains("Allow: GET, HEAD"));
        assert!(request(d.path(), b"GET /site/data/ HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 404"));
        assert!(request(d.path(), b"GET /site/ BAD\r\n\r\n").starts_with("HTTP/1.1 400"));
        for (name, value) in [
            ("x.json", "application/json"),
            ("x.wasm", "application/wasm"),
            ("x.woff2", "font/woff2"),
            ("x.js", "text/javascript; charset=utf-8"),
        ] {
            assert_eq!(mime(Path::new(name)), value);
        }
    }
    #[test]
    fn preview_rejects_headers_over_eight_kib() {
        let d = fixture();
        let mut header = b"GET /site/ HTTP/1.1\r\nX-Fill: ".to_vec();
        header.extend(vec![b'a'; 8192]);
        header.extend(b"\r\n\r\n");
        assert!(request(d.path(), &header).starts_with("HTTP/1.1 431"));
    }
}
