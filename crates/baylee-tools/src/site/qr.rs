//! A web address as a QR code in an SVG file, for a page that must not
//! compute one in the reader's browser (Baylee Central's website: its
//! pages run no script for a picture, and its build takes no new package
//! for three fixed codes). The file is the standard's modules as one path
//! of dark runs on a light square, inside the quiet zone of four modules a
//! reader's camera needs, at the client's own error-correction level
//! (`baylee-client` `lobby/source.rs`). One module is one unit of the
//! `viewBox`; the page gives the size.
//!
//! Only a plain `https://` address without whitespace, control or bidi
//! characters is drawn, the same rule the client's door applies to an
//! address before it opens it or draws it.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use qrcodegen::{QrCode, QrCodeEcc};

/// The light margin round a code, in modules (the standard's four).
const QUIET: i32 = 4;

/// The longest address drawn: a code for more is too fine to scan off a screen.
const LONGEST: usize = 200;

fn plain(address: &str) -> bool {
    let bidi = |c: char| matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}');
    address.len() <= LONGEST
        && address
            .strip_prefix("https://")
            .is_some_and(|rest| !rest.is_empty())
        && !address
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || bidi(c))
}

/// `address` as an SVG document.
///
/// # Errors
/// An address that is not a plain `https://` one, or too long for a code.
pub fn svg(address: &str) -> Result<String> {
    if !plain(address) {
        bail!("only a plain https:// address of at most {LONGEST} characters is drawn");
    }
    let code = QrCode::encode_text(address, QrCodeEcc::Medium)
        .ok()
        .context("the address does not fit a QR code")?;
    let size = code.size();
    let side = size + 2 * QUIET;
    let mut path = String::new();
    for y in 0..size {
        let mut x = 0;
        while x < size {
            if !code.get_module(x, y) {
                x += 1;
                continue;
            }
            let from = x;
            while x < size && code.get_module(x, y) {
                x += 1;
            }
            let run = x - from;
            let _ = write!(path, "M{} {}h{run}v1h-{run}z", from + QUIET, y + QUIET);
        }
    }
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {side} {side}\" \
         shape-rendering=\"crispEdges\"><rect width=\"{side}\" height=\"{side}\" fill=\"#fff\"/>\
         <path fill=\"#000\" d=\"{path}\"/></svg>\n"
    ))
}

/// Writes `address` as a QR code to `out`.
///
/// # Errors
/// An address that is not drawn, or a file that cannot be written.
pub fn run(address: &str, out: &Path) -> Result<()> {
    let document = svg(address)?;
    std::fs::write(out, &document).with_context(|| format!("write {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), document.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dark cells of the document's path, read back from its runs.
    fn drawn(document: &str) -> (i32, Vec<Vec<bool>>) {
        let side: i32 = document
            .split("viewBox=\"0 0 ")
            .nth(1)
            .and_then(|rest| rest.split(' ').next())
            .and_then(|n| n.parse().ok())
            .expect("a viewBox");
        let path = document
            .split(" d=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("a path");
        let mut cells = vec![
            vec![false; usize::try_from(side).expect("small")];
            usize::try_from(side).expect("small")
        ];
        for run in path.split('M').filter(|run| !run.is_empty()) {
            let (at, rest) = run.split_once('h').expect("a run");
            let (x, y) = at.split_once(' ').expect("a start");
            let (x, y): (usize, usize) = (x.parse().expect("x"), y.parse().expect("y"));
            let length: usize = rest
                .split('v')
                .next()
                .expect("a length")
                .parse()
                .expect("a number");
            assert!(rest.ends_with(&format!("v1h-{length}z")), "{run}");
            for cell in &mut cells[y][x..x + length] {
                assert!(!*cell, "a cell drawn twice");
                *cell = true;
            }
        }
        (side, cells)
    }

    /// The file is the standard's encoding of exactly the address, module
    /// for module, and light all round (no decoder here: the encoder is the
    /// reader's other half, as in the client's own test).
    #[test]
    fn a_code_is_the_address_s_modules_inside_the_quiet_zone() {
        for address in [
            "https://github.com/sponsors/AceVik",
            "https://liberapay.com/AceVik",
            "https://ko-fi.com/acevik",
        ] {
            let (side, cells) = drawn(&svg(address).expect("drawn"));
            let code = QrCode::encode_text(address, QrCodeEcc::Medium).expect("encodes");
            assert_eq!(side, code.size() + 2 * QUIET);
            for y in 0..side {
                for x in 0..side {
                    let dark = code.get_module(x - QUIET, y - QUIET);
                    assert_eq!(
                        cells[usize::try_from(y).expect("small")]
                            [usize::try_from(x).expect("small")],
                        dark,
                        "{address}: module ({x}, {y})"
                    );
                }
            }
        }
    }

    #[test]
    fn only_a_plain_https_address_is_drawn() {
        for address in [
            "",
            "https://",
            "http://example.org/",
            "javascript:alert(1)",
            "https://example.org/a b",
            "https://example.org/\u{202e}gpj",
            "https://example.org/\n",
        ] {
            assert!(svg(address).is_err(), "{address:?}");
        }
        assert!(svg(&format!("https://example.org/{}", "a".repeat(LONGEST))).is_err());
    }
}
