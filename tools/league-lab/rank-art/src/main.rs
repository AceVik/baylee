//! Build division badges and the two review plates from the published ladder and league paintings.
mod frames;
mod plate;
mod shaders;

use image::{Rgba, RgbaImage, imageops};
use resvg::{tiny_skia, usvg};
use std::{error::Error, fmt::Write, fs, path::Path};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const SIZE: u32 = 384;
const FAMILIES: [&str; 2] = ["player", "deck"];
// League material, highlight and shadow for the division's enamel clasp.
const MATERIALS: [(&str, &str, &str, &str); 11] = [
    ("Papier", "paper", "#e5d7b7", "#71695b"),
    ("Holz", "wood", "#d4ae7a", "#633e26"),
    ("Eisen", "iron", "#bdc9d0", "#45525e"),
    ("Bronze", "bronze", "#f5b57d", "#89472c"),
    ("Silber", "silver", "#f0f2ed", "#677985"),
    ("Gold", "gold", "#ffe198", "#a8651a"),
    ("Platin", "platinum", "#c9f9f4", "#4c8993"),
    ("Diamant", "diamond", "#a7eaff", "#3967a4"),
    ("Master", "master", "#e1b9ff", "#75539b"),
    ("Grandmaster", "grandmaster", "#ffb4a9", "#93384f"),
    ("Champion", "champion", "#fff0b7", "#b48839"),
];

struct Tier<'a> {
    name: &'static str,
    slug: String,
    source: &'static str,
    light: &'static str,
    shade: &'static str,
    divisions: Vec<&'a str>,
}

fn tiers(ladder: &str) -> Result<Vec<Tier<'_>>> {
    let mut rows = ladder
        .lines()
        .skip(1)
        .map(|r| r.split(',').next().unwrap_or(""))
        .peekable();
    let mut tiers = Vec::new();
    for (name, source, light, shade) in MATERIALS {
        let mut divisions = Vec::new();
        let mut found = false;
        while let Some(&rank) = rows.peek() {
            let (league, division) = rank.split_once(' ').unwrap_or((rank, ""));
            if league != name {
                break;
            }
            if !division.is_empty() {
                divisions.push(division);
            }
            rows.next();
            found = true;
        }
        if !found
            || !matches!(
                divisions.as_slice(),
                [] | ["III", "II", "I"] | ["IV", "III", "II", "I"]
            )
        {
            return Err(format!("unexpected ladder at {name}").into());
        }
        tiers.push(Tier {
            name,
            source,
            light,
            shade,
            slug: name.to_lowercase(),
            divisions,
        });
    }
    if rows.next().is_some() {
        return Err("unknown league after Champion".into());
    }
    Ok(tiers)
}

fn render(opts: &usvg::Options, width: u32, height: u32, body: &str) -> Result<RgbaImage> {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">{body}</svg>"#
    );
    let tree = usvg::Tree::from_str(&svg, opts)?;
    let mut pixels = tiny_skia::Pixmap::new(width, height).ok_or("invalid art dimensions")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    Ok(RgbaImage::from_fn(width, height, |x, y| {
        let c = pixels.pixels()[(y * width + x) as usize].demultiply();
        Rgba([c.red(), c.green(), c.blue(), c.alpha()])
    }))
}

/// Filter premultiplied colour to keep transparent edges free of dark or bright fringes.
fn resize(mut src: RgbaImage, w: u32, h: u32) -> RgbaImage {
    for p in src.pixels_mut() {
        let alpha = u16::from(p[3]);
        for c in &mut p.0[..3] {
            *c = ((u16::from(*c) * alpha + 127) / 255) as u8;
        }
    }
    let mut dst = imageops::resize(&src, w, h, imageops::FilterType::Lanczos3);
    for p in dst.pixels_mut() {
        let a = u16::from(p[3]);
        for c in &mut p.0[..3] {
            *c = (u16::from(*c) * 255 + a / 2)
                .checked_div(a)
                .unwrap_or(0)
                .min(255) as u8;
        }
    }
    dst
}

fn trimmed(path: &Path) -> Result<RgbaImage> {
    let src = image::open(path)?.into_rgba8();
    let (mut left, mut top, mut right, mut bottom) = (src.width(), src.height(), 0, 0);
    for (x, y, _) in src.enumerate_pixels().filter(|(_, _, p)| p[3] >= 96) {
        left = left.min(x);
        right = right.max(x);
        top = top.min(y);
        bottom = bottom.max(y);
    }
    if right <= left || bottom <= top {
        return Err(format!("empty painting: {}", path.display()).into());
    }
    left = left.saturating_sub(3);
    top = top.saturating_sub(3);
    right = (right + 3).min(src.width() - 1);
    bottom = (bottom + 3).min(src.height() - 1);
    Ok(imageops::crop_imm(&src, left, top, right - left + 1, bottom - top + 1).to_image())
}

fn master(path: &Path, tier: usize) -> Result<RgbaImage> {
    let cropped = trimmed(path)?;
    let limit = 308 + 5 * u32::try_from(tier)?;
    let span = cropped.width().max(cropped.height());
    let (w, h) = (
        cropped.width() * limit / span,
        cropped.height() * limit / span,
    );
    let mut dst = RgbaImage::new(SIZE, SIZE);
    imageops::overlay(
        &mut dst,
        &resize(cropped, w, h),
        i64::from((SIZE - w) / 2),
        i64::from((SIZE - h) / 2),
    );
    Ok(dst)
}

fn division(
    opts: &usvg::Options,
    base: &RgbaImage,
    tier: &Tier,
    family: &str,
    step: usize,
) -> Result<RgbaImage> {
    let (hi, lo, roman) = (tier.light, tier.shade, tier.divisions[step]);
    let outline = if family == "player" {
        "M134 301 L250 301 L243 342 L192 365 L141 342 Z"
    } else {
        "M139 300 H245 L254 312 L247 346 L237 355 H147 L137 346 L130 312 Z"
    };
    let mut body = format!(
        r##"
      <defs><linearGradient id="rim" x2=".24" y2="1"><stop stop-color="{hi}"/><stop offset=".23" stop-color="{lo}"/><stop offset=".47" stop-color="{hi}"/><stop offset=".7" stop-color="{lo}"/><stop offset="1" stop-color="{hi}"/></linearGradient>
      <linearGradient id="enamel" x2=".32" y2="1"><stop stop-color="{lo}"/><stop offset=".63" stop-color="#1c2026"/><stop offset="1" stop-color="#10141a"/></linearGradient></defs>"##
    );
    for j in 0..step {
        // Each promotion adds one chased leaf at either edge of the clasp.
        let reach = 28 + 11 * j;
        let y = 337 - 9 * j;
        for flip in ["translate(0 0)", "translate(384 0) scale(-1 1)"] {
            write!(
                body,
                r##"<g transform="{flip}"><path d="M151 {y} q-16 -2 -{reach} -15 q2 17 {reach} 24 Z" fill="url(#rim)" stroke="#19191c" stroke-width="2"/></g>"##
            )?;
        }
    }
    write!(
        body,
        r##"<path d="{outline}" fill="url(#enamel)" stroke="#15191f" stroke-width="10" stroke-linejoin="round"/>
        <path d="{outline}" fill="none" stroke="url(#rim)" stroke-width="6" stroke-linejoin="round"/>
        <text x="192" y="344" font-family="Faustina" font-size="54" font-weight="600" text-anchor="middle" fill="#fff4db" stroke="#13151c" stroke-width="2" paint-order="stroke">{roman}</text>"##
    )?;
    let mut out = base.clone();
    imageops::overlay(&mut out, &render(opts, SIZE, SIZE, &body)?, 0, 0);
    Ok(out)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    if let [action] = args.as_slice()
        && action == "frames"
    {
        return frames::write(&root);
    }
    if let [action, ladder, output] = args.as_slice()
        && action == "shaders"
    {
        return shaders::gallery(&root, Path::new(ladder), Path::new(output));
    }
    if !(1..=2).contains(&args.len()) {
        return Err("usage: baylee-rank-art LADDER_CSV [ORIGINAL_PAINTINGS_DIR] | shaders LADDER_CSV OUTPUT.html | frames".into());
    }
    let out = root.join("art/baylee/ranks");
    let ladder = fs::read_to_string(&args[0])?;
    let tiers = tiers(&ladder)?;
    let mut opts = usvg::Options::default();
    for font in ["Faustina.ttf", "AlegreyaSans-Regular.ttf"] {
        opts.fontdb_mut().load_font_data(fs::read(
            root.join("crates/baylee-client/assets/fonts").join(font),
        )?);
    }
    for family in FAMILIES {
        let dir = out.join(family);
        fs::create_dir_all(&dir)?;
        for (i, tier) in tiers.iter().enumerate() {
            let path = dir.join(format!("{}.png", tier.slug));
            let base = if let Some(originals) = args.get(1) {
                let src = Path::new(originals).join(format!("{family}-{}.png", tier.source));
                let base = master(&src, i)?;
                base.save(&path)?;
                base
            } else {
                image::open(&path)?.into_rgba8()
            };
            for (step, roman) in tier.divisions.iter().enumerate() {
                division(&opts, &base, tier, family, step)?.save(dir.join(format!(
                    "{}-{}.png",
                    tier.slug,
                    roman.to_lowercase()
                )))?;
            }
        }
    }
    if let Some(originals) = args.get(1) {
        for place in 1..=3 {
            let filename = format!("champion-{place}.png");
            let original = Path::new(originals).join(format!("player-{filename}"));
            master(&original, tiers.len() - 1)?.save(out.join("player").join(filename))?;
        }
    }
    plate::write(&opts, &out, &tiers)
}
