//! Rasterizes the same original painting, geometric masks and OFL lettering
//! as make-art.py. Generated assets remain opt-in; CI reads committed assets.
// Fixed artwork coordinates/colors are bounded before integer conversion.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
use super::layout::{APP_X, APPS_X, HEIGHT, ICON_Y, WIDTH};
use anyhow::{Context, Result};
use image::{
    Rgba, RgbaImage,
    imageops::{self, FilterType},
};
use std::{fs, path::Path};
mod formats;
mod raster;
#[cfg(test)]
mod tests;
use raster::{Fonts, text};

const WINDOWS_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
const LINUX_SIZES: [u32; 8] = [16, 24, 32, 48, 64, 128, 256, 512];
const GOLD: [u8; 4] = [243, 213, 138, 255];
const INK: [u8; 4] = [22, 28, 50, 255];

fn body(source: &RgbaImage, side: u32, small: bool) -> RgbaImage {
    let painting = if small {
        let w = f64::from(source.width());
        let [x0, y0, x1, y1] = [0.08, 0.04, 0.92, 0.88].map(|v| (w * v).round_ties_even() as u32);
        imageops::crop_imm(source, x0, y0, x1 - x0, y1 - y0).to_image()
    } else {
        source.clone()
    };
    let mut image = imageops::resize(&painting, side, side, FilterType::Lanczos3);
    if small {
        let blurred = imageops::blur(&image, 1.0);
        for (pixel, blur) in image.pixels_mut().zip(blurred.pixels()) {
            for channel in 0..3 {
                let delta = i32::from(pixel[channel]) - i32::from(blur[channel]);
                if delta.abs() > 1 {
                    pixel[channel] = (f64::from(pixel[channel]) + 0.8 * f64::from(delta))
                        .round_ties_even()
                        .clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
    let mask = raster::mask(side);
    for (pixel, alpha) in image.pixels_mut().zip(mask.pixels()) {
        pixel[3] = alpha[0];
    }
    image
}
fn icon(source: &RgbaImage, size: u32, mac: bool) -> RgbaImage {
    if size <= 32 {
        return body(source, size, true);
    }
    let ratio = if mac { 824.0 / 1024.0 } else { 0.94 };
    let mut inner = (f64::from(size) * ratio).round_ties_even() as u32;
    inner -= (size - inner) % 2;
    let at = (size - inner) / 2;
    let mut canvas = RgbaImage::new(size, size);
    if mac {
        let mut shadow = image::GrayImage::new(size, size);
        imageops::overlay(
            &mut shadow,
            &raster::mask(inner),
            i64::from(at),
            i64::from(at) + (f64::from(size) * 0.012).round_ties_even() as i64,
        );
        let shadow = imageops::blur(&shadow, size as f32 * 0.014);
        for (pixel, alpha) in canvas.pixels_mut().zip(shadow.pixels()) {
            pixel[3] = (f64::from(alpha[0]) * 0.45).round_ties_even() as u8;
        }
    }
    imageops::overlay(
        &mut canvas,
        &body(source, inner, false),
        i64::from(at),
        i64::from(at),
    );
    canvas
}
fn background(scale: u32, fonts: &Fonts) -> RgbaImage {
    let mut image = RgbaImage::new(WIDTH * scale, HEIGHT * scale);
    for (_, y, pixel) in image.enumerate_pixels_mut() {
        let t = f64::from(y) / f64::from(HEIGHT * scale - 1);
        let (a, b, t) = if t < 0.62 {
            ([20, 27, 52], [92, 108, 156], t / 0.62)
        } else {
            ([92, 108, 156], [128, 140, 182], (t - 0.62) / 0.38)
        };
        *pixel = Rgba([0, 0, 0, 255]);
        for c in 0..3 {
            pixel[c] = (f64::from(a[c]) + f64::from(b[c] - a[c]) * t).round_ties_even() as u8;
        }
    }
    let s = f64::from(scale);
    for (x, y, r) in [
        (58.0, 34.0, 1.4),
        (132.0, 70.0, 1.0),
        (240.0, 26.0, 1.2),
        (402.0, 58.0, 1.0),
        (520.0, 30.0, 1.5),
        (604.0, 82.0, 1.1),
        (318.0, 96.0, 0.9),
        (88.0, 118.0, 0.9),
    ] {
        raster::ellipse(&mut image, (x * s, y * s), r * s, GOLD);
    }
    text(
        &mut image,
        &fonts.medium,
        26.0 * scale as f32,
        (f64::from(WIDTH) * s / 2.0, 52.0 * s),
        "Drag Baylee to Applications",
        GOLD,
        true,
    );
    let y = f64::from(ICON_Y) * s;
    let x0 = f64::from(APP_X + 74) * s;
    let x1 = f64::from(APPS_X - 74) * s;
    raster::polygon(
        &mut image,
        &[
            (x0, y - 2.5 * s),
            (x1 - 16.0 * s, y - 2.5 * s),
            (x1 - 16.0 * s, y + 2.5 * s),
            (x0, y + 2.5 * s),
        ],
        GOLD,
    );
    raster::polygon(
        &mut image,
        &[
            (x1, y),
            (x1 - 18.0 * s, y - 12.0 * s),
            (x1 - 18.0 * s, y + 12.0 * s),
        ],
        GOLD,
    );
    text(
        &mut image,
        &fonts.regular,
        12.0 * scale as f32,
        (f64::from(WIDTH) * s / 2.0, f64::from(HEIGHT - 22) * s),
        "Unofficial fan content, not affiliated with Wizards of the Coast.",
        INK,
        true,
    );
    image
}
fn review(source: &RgbaImage, fonts: &Fonts, directory: &Path) -> Result<()> {
    const SIZES: [u32; 5] = [256, 128, 64, 32, 16];
    let width = 24 + SIZES.iter().map(|n| n + 24).sum::<u32>();
    let row_h = 334;
    let mut sheet = RgbaImage::new(width * 2, row_h * 3 + 50);
    for (g, ground) in [[18, 22, 40, 255], [236, 238, 242, 255]]
        .into_iter()
        .enumerate()
    {
        let start = u32::try_from(g)? * width;
        for (x, _, pixel) in sheet.enumerate_pixels_mut() {
            if x >= start && x < start + width {
                *pixel = Rgba(ground);
            }
        }
        let ink = if g == 0 {
            [240, 240, 240, 255]
        } else {
            [20, 20, 30, 255]
        };
        text(
            &mut sheet,
            &fonts.medium,
            22.0,
            (f64::from(start + 24), 12.0),
            if g == 0 {
                "dark ground"
            } else {
                "light ground"
            },
            ink,
            false,
        );
        for (row, label) in [
            "before (square)",
            "after: Windows / Linux",
            "after: macOS (Apple grid)",
        ]
        .into_iter()
        .enumerate()
        {
            let y = 50 + u32::try_from(row)? * row_h;
            text(
                &mut sheet,
                &fonts.medium,
                22.0,
                (f64::from(start + 24), f64::from(y)),
                label,
                ink,
                false,
            );
            let mut x = start + 24;
            for n in SIZES {
                let picture = if row == 0 {
                    imageops::resize(source, n, n, FilterType::Lanczos3)
                } else {
                    icon(source, n, row == 2)
                };
                imageops::overlay(
                    &mut sheet,
                    &picture,
                    i64::from(x),
                    i64::from(y + 34 + 256 - n),
                );
                text(
                    &mut sheet,
                    &fonts.regular,
                    16.0,
                    (f64::from(x), f64::from(y + 34 + 256 + 4)),
                    &format!("{n}px"),
                    ink,
                    false,
                );
                x += n + 24;
            }
        }
    }
    fs::create_dir_all(directory)?;
    sheet.save(directory.join("icon-old-vs-new.png"))?;
    icon(source, 1024, true).save(directory.join("new-macos-1024.png"))?;
    icon(source, 512, false).save(directory.join("new-flat-512.png"))?;
    Ok(())
}

pub(super) fn generate(root: &Path, review_dir: Option<&Path>) -> Result<()> {
    let brand = root.join("crates/baylee-client/assets/brand");
    let source = image::open(brand.join("baylee-icon.png"))
        .context("reading original brand painting")?
        .to_rgba8();
    let fonts = Fonts::load(&root.join("crates/baylee-client/assets/fonts"))?;
    let art = root.join("scripts/installers");
    background(1, &fonts).save(art.join("macos/background.png"))?;
    background(2, &fonts).save(art.join("macos/background@2x.png"))?;
    fs::write(
        brand.join("baylee.icns"),
        formats::icns(|n| icon(&source, n, true))?,
    )?;
    icon(&source, 128, false).save(brand.join("baylee-window.png"))?;
    icon(&source, 256, false).save(art.join("linux/baylee.png"))?;
    for n in LINUX_SIZES {
        let path = art.join(format!("linux/hicolor/{n}x{n}/apps/baylee.png"));
        fs::create_dir_all(path.parent().unwrap())?;
        icon(&source, n, false).save(path)?;
    }
    let images: Vec<_> = WINDOWS_SIZES
        .into_iter()
        .map(|n| icon(&source, n, false))
        .collect();
    fs::write(art.join("windows/baylee.ico"), formats::ico(&images)?)?;
    if let Some(directory) = review_dir {
        review(&source, &fonts, directory)?;
    }
    Ok(())
}
