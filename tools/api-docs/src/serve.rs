//! `serve`: the documents in `docs/api/` in Swagger UI, on this machine
//! only. The page loads Swagger UI from jsDelivr at a pinned version with
//! subresource integrity (owner, 21.10.2026: no copy in the repository), and
//! this server sends only the page and the documents. It binds the loopback
//! address and answers only a loopback `Host`, so a page elsewhere that
//! rebinds a name to 127.0.0.1 is answered nothing. "Try it out" talks to a
//! local gateway, whose CORS answers any origin and takes a bearer token.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::Path;

/// Where it listens unless `--port` says otherwise: beside the gateway's
/// 28766 and its admin console's 28767.
pub const PORT: u16 = 28768;

/// The pinned Swagger UI and the SHA-256 jsDelivr lists for each file.
const SWAGGER_UI: &str = "https://cdn.jsdelivr.net/npm/swagger-ui-dist@5.33.1";
const CSS_SRI: &str = "sha256-GsMk99zSfkuThrS9ZCEnHsFH6SKiLAW6JLEVFemqYyE=";
const BUNDLE_SRI: &str = "sha256-BQvEFe5wSNzYgWgmePcgJk59peNz90YdfFjHVTBSVfc=";
const PRESET_SRI: &str = "sha256-UkPUkuFFBeDKuHrIsBldDmFZQ+ZRdDsraYRQpG60cL4=";

/// Serves until killed.
pub fn run(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
    eprintln!("api-docs: http://127.0.0.1:{port}/ (Ctrl-C ends it)");
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        if let Err(error) = answer(stream, port) {
            eprintln!("api-docs: {error}");
        }
    }
    Ok(())
}

fn answer(mut stream: TcpStream, port: u16) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    let mut host = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("host")
        {
            host = Some(value.trim().to_owned());
        }
    }
    let loopback = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    if !host.is_some_and(|host| loopback.contains(&host)) {
        return respond(
            &mut stream,
            "403 Forbidden",
            "text/plain",
            b"loopback only\n",
        );
    }
    let mut parts = request.split_whitespace();
    let (Some("GET"), Some(target)) = (parts.next(), parts.next()) else {
        return respond(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            b"GET only\n",
        );
    };
    // Swagger UI keeps the chosen document in the query (`?urls.primaryName=`).
    let path = target.split_once('?').map_or(target, |(path, _)| path);
    if path == "/" {
        return respond(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            page()?.as_bytes(),
        );
    }
    match path.strip_prefix("/docs/").filter(|name| is_document(name)) {
        Some(name) => match std::fs::read(Path::new("docs/api").join(name)) {
            Ok(body) => respond(
                &mut stream,
                "200 OK",
                "application/yaml; charset=utf-8",
                &body,
            ),
            Err(_) => respond(
                &mut stream,
                "404 Not Found",
                "text/plain",
                b"no such document\n",
            ),
        },
        None => respond(&mut stream, "404 Not Found", "text/plain", b"not found\n"),
    }
}

/// A document's file name as `docs/api/` holds them, and nothing that could
/// name another file.
fn is_document(name: &str) -> bool {
    let stem = name
        .strip_suffix(".openapi.yaml")
        .or_else(|| name.strip_suffix(".asyncapi.yaml"));
    stem.is_some_and(|stem| {
        !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
    })
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

/// The page: every `*.openapi.yaml` in the picker, every `*.asyncapi.yaml`
/// as a link (Swagger UI does not draw those).
fn page() -> std::io::Result<String> {
    let mut openapi = Vec::new();
    let mut asyncapi = Vec::new();
    for entry in std::fs::read_dir("docs/api")? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.ends_with(".openapi.yaml") && is_document(&name) {
            openapi.push(name);
        } else if name.ends_with(".asyncapi.yaml") && is_document(&name) {
            asyncapi.push(name);
        }
    }
    // The gateway's public document first: the one a client author opens
    // the page for.
    openapi.sort_by_key(|name| (name != "gateway.openapi.yaml", name.clone()));
    asyncapi.sort();
    let urls = openapi
        .iter()
        .map(|name| {
            format!(
                "{{url: \"docs/{name}\", name: \"{}\"}}",
                name.trim_end_matches(".openapi.yaml")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let links = asyncapi
        .iter()
        .map(|name| format!("<a href=\"docs/{name}\">{name}</a>"))
        .collect::<Vec<_>>()
        .join(" · ");
    Ok(format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Baylee API</title>
<link rel="stylesheet" href="{SWAGGER_UI}/swagger-ui.css" integrity="{CSS_SRI}" crossorigin="anonymous">
<style>body {{ margin: 0; }} .sockets {{ font: 14px sans-serif; padding: 8px 16px; }}</style>
</head>
<body>
<div class="sockets">Sockets (AsyncAPI): {links}</div>
<div id="ui"></div>
<script src="{SWAGGER_UI}/swagger-ui-bundle.js" integrity="{BUNDLE_SRI}" crossorigin="anonymous"></script>
<script src="{SWAGGER_UI}/swagger-ui-standalone-preset.js" integrity="{PRESET_SRI}" crossorigin="anonymous"></script>
<script>
window.ui = SwaggerUIBundle({{
  urls: [{urls}],
  dom_id: "#ui",
  presets: [SwaggerUIBundle.presets.apis, SwaggerUIStandalonePreset],
  layout: "StandaloneLayout",
  persistAuthorization: false,
}});
</script>
</body>
</html>
"##
    ))
}

#[cfg(test)]
mod tests {
    use super::is_document;

    #[test]
    fn only_a_document_name_is_served() {
        assert!(is_document("gateway.openapi.yaml"));
        assert!(is_document("gateway-admin.openapi.yaml"));
        assert!(is_document("gateway.asyncapi.yaml"));
        for name in [
            "../Cargo.toml",
            ".openapi.yaml",
            "a/b.openapi.yaml",
            "x.yaml",
            "X.openapi.yaml",
        ] {
            assert!(!is_document(name), "{name}");
        }
    }
}
