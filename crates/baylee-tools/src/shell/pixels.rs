//! Contrast sampling excludes glyphs, borders and rounded corners, as before.
use super::tree::{Node, Tree};
use anyhow::{Result, ensure};
use image::RgbImage;
use serde::Serialize;
use std::path::Path;
pub(super) const INK: [f64; 3] = [0.90, 0.93, 0.94];
pub(super) const MUTED: [f64; 3] = [0.58, 0.64, 0.68];
pub(super) fn luminance(rgb: [f64; 3]) -> f64 {
    let [r, g, b] = rgb.map(|c| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}
pub(super) fn ratio(a: [f64; 3], b: [f64; 3]) -> f64 {
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn rgb(image: &RgbImage, x: u32, y: u32) -> [f64; 3] {
    image.get_pixel(x, y).0.map(|c| f64::from(c) / 255.0)
}
pub(super) fn load(path: &Path) -> Result<RgbImage> {
    let image = image::open(path)?.into_rgb8();
    ensure!(
        not_black(&image),
        "screenshot is black; no render measurement"
    );
    Ok(image)
}
pub(super) fn not_black(image: &RgbImage) -> bool {
    // Pillow's RGB -> L uses these coefficients (not linear luminance).
    image.pixels().any(|p| {
        0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2]) >= 23.5
    })
}
#[derive(Debug, Serialize)]
pub(super) struct Contrast {
    pub kind: String,
    pub ink: f64,
    pub muted: f64,
    pub samples: usize,
}
fn near(n: &Node, x: f64, y: f64) -> bool {
    n.x - 2.0 <= x && x <= n.right() + 2.0 && n.y - 2.0 <= y && y <= n.bottom() + 2.0
}
fn covered(cover: &[&Node], edges: &[&Node], x: f64, y: f64) -> bool {
    cover.iter().any(|d| near(d, x, y))
        || edges.iter().any(|d| {
            near(d, x, y)
                && ((x - d.x).abs().min((x - d.right()).abs()) <= 4.0
                    || (y - d.y).abs().min((y - d.bottom()).abs()) <= 4.0)
        })
}
pub(super) fn contrast(
    tree: &Tree,
    image: &RgbImage,
    width: f64,
    height: f64,
) -> Result<(Vec<String>, Vec<Contrast>)> {
    ensure!(
        width > 0.0 && height > 0.0 && width.is_finite() && height.is_finite(),
        "invalid canvas size"
    );
    if !not_black(image) {
        return Ok((
            vec!["contrast: black frame; no measurement".into()],
            Vec::new(),
        ));
    }
    let sx = f64::from(image.width()) / width;
    let sy = f64::from(image.height()) / height;
    let mut faults = Vec::new();
    let mut report = Vec::new();
    for n in &tree.0 {
        if !matches!(n.kind(), "panel" | "header" | "mist" | "opaque")
            || tree
                .owner(n, &["scroll"])
                .is_some_and(|p| !n.inside(p, 1.0))
        {
            continue;
        }
        let x0 = n.x.max(0.0);
        let y0 = n.y.max(0.0);
        let x1 = n.right().min(width);
        let y1 = n.bottom().min(height);
        if x1 - x0 < 4.0 || y1 - y0 < 4.0 {
            continue;
        }
        // Classify once per surface, not once per sampled pixel.
        let below = tree.descendants(n.index);
        let parents: std::collections::BTreeSet<_> =
            below.iter().filter_map(|d| d.parent).collect();
        let (cover, edges): (Vec<_>, Vec<_>) = below
            .into_iter()
            .partition(|d| d.t.is_some() || d.role_present || !parents.contains(&d.index));
        let edges: Vec<_> = edges
            .into_iter()
            .filter(|d| d.w > 0.0 && d.h > 0.0)
            .collect();
        let mut brightest = None;
        let mut seen = 0;
        let mut y = y0 + 3.0;
        while y < y1 - 3.0 {
            let mut x = x0 + 3.0;
            while x < x1 - 3.0 {
                let corner = (x - n.x < 16.0 || n.right() - x < 16.0)
                    && (y - n.y < 16.0 || n.bottom() - y < 16.0);
                if !corner && !covered(&cover, &edges, x, y) {
                    let color = rgb(image, (x * sx) as u32, (y * sy) as u32);
                    seen += 1;
                    if brightest.is_none_or(|b| luminance(color) > luminance(b)) {
                        brightest = Some(color);
                    }
                }
                x += 3.0;
            }
            y += 3.0;
        }
        if let Some(brightest) = brightest {
            let ink = ratio(INK, brightest);
            let muted = ratio(MUTED, brightest);
            if ink < 7.0 {
                faults.push(format!(
                    "contrast: ink {ink:.2} on {} {}",
                    n.kind(),
                    n.rect()
                ));
            }
            if n.kind() != "mist" && muted < 4.5 {
                faults.push(format!(
                    "contrast: muted {muted:.2} on {} {}",
                    n.kind(),
                    n.rect()
                ));
            }
            report.push(Contrast {
                kind: n.kind().into(),
                ink: (ink * 100.0).round() / 100.0,
                muted: (muted * 100.0).round() / 100.0,
                samples: seen,
            });
        }
    }
    Ok((faults, report))
}
pub(super) fn credit(tree: &Tree, image: &RgbImage) -> Option<f64> {
    let mut worst: Option<f64> = None;
    for n in tree
        .0
        .iter()
        .filter(|n| n.kind() == "credit" && n.w >= 4.0 && n.parent.is_some())
    {
        let plate = &tree.0[n.parent?];
        if tree
            .owner(n, &["panel"])
            .is_some_and(|p| !plate.inside(p, -8.0))
        {
            continue;
        }
        let mut brightest = None;
        for x in (plate.x as i32 + 1)..(plate.right() as i32 - 1) {
            if (n.x - 3.0..=n.right() + 3.0).contains(&f64::from(x)) {
                continue;
            }
            for y in (plate.y as i32 + 2)..(plate.bottom() as i32 - 2) {
                if x < 0
                    || y < 0
                    || i64::from(x) >= i64::from(image.width())
                    || i64::from(y) >= i64::from(image.height())
                {
                    continue;
                }
                let color = rgb(image, x as u32, y as u32);
                if brightest.is_none_or(|b| luminance(color) > luminance(b)) {
                    brightest = Some(color);
                }
            }
        }
        if let Some(b) = brightest {
            let r = ratio(MUTED, b);
            worst = Some(worst.map_or(r, |w| w.min(r)));
        }
    }
    worst
}
pub(super) fn ring(image: &RgbImage, n: &Node) -> usize {
    let mut hits = 0;
    for x in (n.x as i32 - 4)..(n.right() as i32 + 4) {
        for y in [n.y as i32 - 3, n.bottom() as i32 + 2] {
            if x < 0
                || y < 0
                || i64::from(x) >= i64::from(image.width())
                || i64::from(y) >= i64::from(image.height())
            {
                continue;
            }
            let [r, g, b] = image.get_pixel(x as u32, y as u32).0;
            if g > 150 && b > 140 && r < 130 {
                hits += 1;
            }
        }
    }
    hits
}
