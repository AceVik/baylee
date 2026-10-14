//! Small raster primitives and OFL font rendering, with no platform runtime.
// Coordinates are bounded by the fixed artwork sizes (at most 1320 pixels).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
use ab_glyph::{Font, FontArc, ScaleFont, point};
use anyhow::{Result, anyhow};
use image::{GrayImage, Luma, Rgba, RgbaImage};
use std::{fs, path::Path};

pub(super) struct Fonts {
    pub regular: FontArc,
    pub medium: FontArc,
}
impl Fonts {
    pub fn load(directory: &Path) -> Result<Self> {
        Ok(Self {
            regular: FontArc::try_from_vec(fs::read(directory.join("AlegreyaSans-Regular.ttf"))?)
                .map_err(|e| anyhow!("font: {e}"))?,
            medium: FontArc::try_from_vec(fs::read(directory.join("AlegreyaSans-Medium.ttf"))?)
                .map_err(|e| anyhow!("font: {e}"))?,
        })
    }
}
pub(super) fn mask(side: u32) -> GrayImage {
    let corner = f64::from(side) * 0.40;
    GrayImage::from_fn(side, side, |x, y| {
        let mut covered = 0;
        // Same exponent-four rounded-square geometry, at 4x supersampling.
        for dy in 0..4 {
            for dx in 0..4 {
                let px = f64::from(x) + (f64::from(dx) + 0.5) / 4.0;
                let py = f64::from(y) + (f64::from(dy) + 0.5) / 4.0;
                let qx = ((px - f64::from(side) / 2.0).abs() - (f64::from(side) / 2.0 - corner))
                    .max(0.0)
                    / corner;
                let qy = ((py - f64::from(side) / 2.0).abs() - (f64::from(side) / 2.0 - corner))
                    .max(0.0)
                    / corner;
                if qx.powi(4) + qy.powi(4) <= 1.0 {
                    covered += 1;
                }
            }
        }
        Luma([(f64::from(covered) * 255.0 / 16.0).round_ties_even() as u8])
    })
}
pub(super) fn ellipse(image: &mut RgbaImage, centre: (f64, f64), radius: f64, colour: [u8; 4]) {
    let (cx, cy) = centre;
    for y in ((cy - radius).floor().max(0.0) as u32)
        ..=((cy + radius).ceil() as u32).min(image.height() - 1)
    {
        for x in ((cx - radius).floor().max(0.0) as u32)
            ..=((cx + radius).ceil() as u32).min(image.width() - 1)
        {
            if (f64::from(x) - cx).powi(2) + (f64::from(y) - cy).powi(2) <= radius * radius {
                image.put_pixel(x, y, Rgba(colour));
            }
        }
    }
}
pub(super) fn polygon(image: &mut RgbaImage, points: &[(f64, f64)], colour: [u8; 4]) {
    let min_x = points
        .iter()
        .map(|p| p.0)
        .fold(f64::INFINITY, f64::min)
        .floor()
        .max(0.0) as u32;
    let max_x =
        (points.iter().map(|p| p.0).fold(0.0, f64::max).ceil() as u32).min(image.width() - 1);
    let min_y = points
        .iter()
        .map(|p| p.1)
        .fold(f64::INFINITY, f64::min)
        .floor()
        .max(0.0) as u32;
    let max_y =
        (points.iter().map(|p| p.1).fold(0.0, f64::max).ceil() as u32).min(image.height() - 1);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (xv, yv) = (f64::from(x), f64::from(y));
            let mut inside = false;
            for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
                if (a.1 > yv) != (b.1 > yv) && xv < (b.0 - a.0) * (yv - a.1) / (b.1 - a.1) + a.0 {
                    inside = !inside;
                }
            }
            if inside {
                image.put_pixel(x, y, Rgba(colour));
            }
        }
    }
}
#[allow(clippy::too_many_arguments)] // A drawing primitive's font, anchor and paint.
pub(super) fn text(
    image: &mut RgbaImage,
    font: &FontArc,
    size: f32,
    at: (f64, f64),
    words: &str,
    colour: [u8; 4],
    centred: bool,
) {
    let px = size * font.height_unscaled() / font.units_per_em().unwrap_or(1000.0);
    let scaled = font.as_scaled(px);
    let mut pen = 0.0;
    let mut previous = None;
    let mut glyphs = Vec::new();
    for c in words.chars() {
        let id = scaled.glyph_id(c);
        if let Some(before) = previous {
            pen += scaled.kern(before, id);
        }
        let mut glyph = id.with_scale(px);
        glyph.position = point(pen, 0.0);
        glyphs.push(glyph);
        pen += scaled.h_advance(id);
        previous = Some(id);
    }
    let baseline = if centred {
        at.1 as f32 + scaled.ascent().midpoint(scaled.descent())
    } else {
        at.1 as f32 + scaled.ascent()
    };
    let start = at.0 as f32 - if centred { pen / 2.0 } else { 0.0 };
    for mut glyph in glyphs {
        glyph.position.x += start;
        glyph.position.y += baseline;
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            outline.draw(|x, y, coverage| {
                let x = i64::from(x) + bounds.min.x as i64;
                let y = i64::from(y) + bounds.min.y as i64;
                if x >= 0 && y >= 0 && x < i64::from(image.width()) && y < i64::from(image.height())
                {
                    let target = image.get_pixel_mut(x as u32, y as u32);
                    for c in 0..3 {
                        target[c] = (f32::from(target[c]) * (1.0 - coverage)
                            + f32::from(colour[c]) * coverage)
                            .round() as u8;
                    }
                }
            });
        }
    }
}
