use super::{FAMILIES, Result, Tier, render, resize};
use image::imageops;
use resvg::usvg;
use std::{
    fmt::Write,
    path::{Path, PathBuf},
};

struct Tile {
    path: PathBuf,
    x: i64,
    y: i64,
    size: u32,
}

fn header(title: &str, note: &str) -> String {
    format!(
        r##"<defs><linearGradient id="ground" x2=".63" y2="1"><stop stop-color="#17202a"/><stop offset=".48" stop-color="#10161e"/><stop offset="1" stop-color="#171721"/></linearGradient></defs>
        <rect width="100%" height="100%" fill="url(#ground)"/>
        <text x="48" y="78" font-family="Faustina" font-size="52" fill="#f7ead2">{title}</text>
        <text x="50" y="115" font-family="Alegreya Sans" font-size="25" fill="#c7c8c7">{note}</text>
        <path d="M48 141 H1408" stroke="#43505b"/>"##
    )
}

fn label(body: &mut String, x: u32, y: u32, name: &str, center: bool) -> std::fmt::Result {
    let anchor = if center { "middle" } else { "start" };
    write!(
        body,
        r##"<text x="{x}" y="{y}" text-anchor="{anchor}" font-family="Faustina" font-size="31" fill="#f4e9d7">{name}</text>"##
    )
}

fn pair(tiles: &mut Vec<Tile>, out: &Path, stem: &str, x: u32, y: u32, size: u32) {
    for (slot, family) in FAMILIES.iter().enumerate() {
        tiles.push(Tile {
            path: out.join(family).join(format!("{stem}.png")),
            x: i64::from(x) + i64::try_from(slot).expect("two badge families") * i64::from(size),
            y: y.into(),
            size,
        });
    }
}

fn save(opts: &usvg::Options, body: &str, tiles: Vec<Tile>, out: &Path, height: u32) -> Result<()> {
    let mut sheet = render(opts, 1456, height, body)?;
    for tile in tiles {
        let icon = resize(image::open(tile.path)?.into_rgba8(), tile.size, tile.size);
        imageops::overlay(&mut sheet, &icon, tile.x, tile.y);
    }
    sheet.save(out)?;
    Ok(())
}

pub(super) fn write(opts: &usvg::Options, out: &Path, tiers: &[Tier]) -> Result<()> {
    let mut body = header(
        "Die Rangabzeichen",
        "Spieler-Elo links   ·   Deck-Elo rechts",
    );
    let mut tiles = Vec::new();
    for (row, group) in tiers.chunks(4).enumerate() {
        let y = 165 + 232 * u32::try_from(row)?;
        let inset = (1456 - 344 * u32::try_from(group.len())?) / 2;
        for (col, tier) in group.iter().enumerate() {
            let x = inset + 344 * u32::try_from(col)?;
            write!(
                body,
                r#"<rect x="{}" y="{y}" width="330" height="216" rx="16" fill="{}" fill-opacity=".065" stroke="{}" stroke-opacity=".25"/>"#,
                x + 7,
                tier.light,
                tier.light
            )?;
            pair(&mut tiles, out, &tier.slug, x + 12, y + 7, 160);
            label(&mut body, x + 172, y + 200, tier.name, true)?;
        }
    }
    save(opts, &body, tiles, &out.join("ligen.png"), 870)?;

    let mut body = header(
        "Die Divisionen",
        "Aufstieg von links nach rechts   ·   Je Paar zuerst das Spielerabzeichen, dann das Deckabzeichen",
    );
    let mut tiles = Vec::new();
    let mut y = 168;
    for tier in tiers.iter().filter(|t| !t.divisions.is_empty()) {
        label(&mut body, 48, y + 92, tier.name, false)?;
        for (step, &roman) in tier.divisions.iter().enumerate() {
            let x = 202 + 300 * u32::try_from(step)?;
            write!(
                body,
                r#"<rect x="{x}" y="{y}" width="280" height="166" rx="13" fill="{}" fill-opacity=".065" stroke="{}" stroke-opacity=".22"/>"#,
                tier.light, tier.light
            )?;
            let stem = format!("{}-{}", tier.slug, roman.to_lowercase());
            pair(&mut tiles, out, &stem, x + 11, y + 3, 129);
            label(&mut body, x + 140, y + 154, roman, true)?;
        }
        y += 181;
    }
    save(opts, &body, tiles, &out.join("divisionen.png"), y + 12)?;

    let mut body = header(
        "Die Champion-Helme",
        "Spieler-Elo   ·   Das Tagesranking bestimmt, wer die drei besonderen Helme trägt.",
    );
    let mut tiles = Vec::new();
    for (column, (stem, caption, light)) in [
        ("champion", "Plätze 4–100", "#c4b79a"),
        ("champion-3", "Platz 3", "#da9568"),
        ("champion-2", "Platz 2", "#c0d9ef"),
        ("champion-1", "Platz 1", "#ffda88"),
    ]
    .into_iter()
    .enumerate()
    {
        let x = 40 + 346 * u32::try_from(column)?;
        write!(
            body,
            r#"<rect x="{x}" y="165" width="336" height="365" rx="18" fill="{light}" fill-opacity=".075" stroke="{light}" stroke-opacity=".43"/>"#
        )?;
        tiles.push(Tile {
            path: out.join("player").join(format!("{stem}.png")),
            x: (x + 8).into(),
            y: 172,
            size: 320,
        });
        label(&mut body, x + 168, 511, caption, true)?;
    }
    save(opts, &body, tiles, &out.join("podium.png"), 555)
}
