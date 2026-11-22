//! Original profile-frame engravings. The exporter paints at the game's 384 px master size.
//! Each league has a different outer silhouette; the negative space is a real transparent cut.
use super::{Result, render};
use resvg::usvg;
use std::{fmt::Write, fs, path::Path};

mod ornament;
mod podium;
const LEAGUES: [(&str, u8); 11] = [
    ("papier", 0),
    ("holz", 3),
    ("eisen", 3),
    ("bronze", 3),
    ("silber", 3),
    ("gold", 4),
    ("platin", 4),
    ("diamant", 4),
    ("master", 0),
    ("grandmaster", 0),
    ("champion", 0),
];
// Glancing highlight, warm face, shadow, enamel highlight and enamel shadow.
const PALETTE: [[&str; 5]; 11] = [
    ["#fff3ca", "#d7b681", "#735b40", "#f5d69c", "#9d6d47"],
    ["#edc58a", "#af7947", "#513521", "#a7bf77", "#496044"],
    ["#d0e1e9", "#8c9daa", "#364355", "#c4d7dc", "#495b70"],
    ["#ffe1af", "#ca8756", "#693b2b", "#85c3ae", "#365e57"],
    ["#fbffff", "#d1dbeb", "#63758b", "#eff8ff", "#7aa6ca"],
    ["#fff6af", "#e8ae44", "#8f591c", "#ffdf77", "#b6711b"],
    ["#f5fffa", "#c3ddd6", "#587e84", "#9cf9df", "#249e9f"],
    ["#d6faff", "#8abbe0", "#31557e", "#92fdff", "#1462ba"],
    ["#ffe2a1", "#ce9965", "#654234", "#cd8eff", "#64278f"],
    ["#ffe6bd", "#dfa67e", "#744135", "#ff8996", "#a11639"],
    ["#fff2bb", "#e4b968", "#805528", "#fff7dc", "#83b5c4"],
];

#[derive(Clone, Copy)]
struct Mark {
    tier: usize,
    deck: bool,
    step: u8,
    place: u8,
}
impl Mark {
    fn inner(self) -> i32 {
        match self.tier {
            0..=1 => 149,
            2..=4 => 141,
            5..=7 => 134,
            _ => 123,
        }
    }
    fn colors(self) -> [&'static str; 5] {
        match self.place {
            3 => ["#fff0d4", "#d49c7b", "#79524a", "#ffb3b3", "#b26373"],
            2 => ["#ffffff", "#c1d8ed", "#61738f", "#99d9ff", "#2256ad"],
            _ => PALETTE[self.tier],
        }
    }
}

fn path(out: &mut String, d: &str, fill: &str) {
    write!(out,r##"<path d="{d}" fill="{fill}" stroke="#25262c" stroke-width="2.7" stroke-linejoin="round"/>"##).unwrap();
}
fn stroke(out: &mut String, d: &str, paint: &str, width: f32) {
    write!(out,r#"<path d="{d}" fill="none" stroke="{paint}" stroke-width="{width}" stroke-linejoin="round" stroke-linecap="round"/>"#).unwrap();
}
fn dot(out: &mut String, x: i32, y: i32, r: i32, fill: &str) {
    write!(
        out,
        r##"<circle cx="{x}" cy="{y}" r="{r}" fill="{fill}" stroke="#37313a" stroke-width="2"/>"##
    )
    .unwrap();
}
fn flip(out: &mut String, draw: impl Fn(&mut String)) {
    for mirror in [1, -1] {
        write!(out, r#"<g transform="scale({mirror} 1)">"#).unwrap();
        draw(out);
        out.push_str("</g>");
    }
}
fn turn(out: &mut String, angle: i32, draw: impl FnOnce(&mut String)) {
    write!(out, r#"<g transform="rotate({angle})">"#).unwrap();
    draw(out);
    out.push_str("</g>");
}
const METAL: &str = "url(#metal)";
const EDGE: &str = "url(#edge)";
const GEM: &str = "url(#gem)";
const IVORY: &str = "url(#ivory)";

fn defs(m: Mark) -> String {
    let [hi, middle, dark, gem, deep] = m.colors();
    format!(
        r##"<defs>
 <linearGradient id="metal" x1="0" y1="0" x2=".9" y2="1" gradientUnits="objectBoundingBox">
 <stop stop-color="{hi}"/><stop offset=".22" stop-color="{middle}"/><stop offset=".43" stop-color="{hi}"/><stop offset=".57" stop-color="{middle}"/><stop offset=".85" stop-color="{dark}"/><stop offset="1" stop-color="{middle}"/></linearGradient>
 <linearGradient id="edge" x2="0.8" y2="1"><stop stop-color="{dark}"/><stop offset=".34" stop-color="{middle}"/><stop offset=".57" stop-color="{dark}"/><stop offset=".86" stop-color="{middle}"/><stop offset="1" stop-color="{dark}"/></linearGradient>
 <linearGradient id="gem" x2="0.6" y2="1"><stop stop-color="#ffffff"/><stop offset=".26" stop-color="{gem}"/><stop offset=".68" stop-color="{deep}"/><stop offset="1" stop-color="{gem}"/></linearGradient>
 <linearGradient id="ivory" x2="0.9" y2=".8"><stop stop-color="#fffdf0"/><stop offset=".43" stop-color="#e4e7ef"/><stop offset=".65" stop-color="#9eb9bb"/><stop offset="1" stop-color="#f2e7d6"/></linearGradient>
 </defs>"##
    )
}

fn octagon(r: i32, cut: i32) -> String {
    let n = -r;
    let side = r - cut;
    let neg = -side;
    format!("M{neg} {n} H{side} L{r} {neg} V{side} L{side} {r} H{neg} L{n} {side} V{neg} Z")
}
fn outline(m: Mark, r: i32) -> String {
    if m.deck {
        return octagon(
            r,
            match m.tier {
                0..=1 => 14,
                2 => 37,
                3..=5 => 22,
                6 => 42,
                7 => 52,
                _ => 33,
            },
        );
    }
    match m.tier {
        0 => format!(
            "M0 -{r} Q83 -{r} 125 -85 L{r} -37 L{r} 64 Q99 {r} 0 {r} Q-90 {r} -{r} 62 L-{r} -37 L-116 -99 Q-80 -{r} 0 -{r}Z"
        ),
        2 => octagon(r, (r * 58) / 100),
        6 => format!(
            "M0 -{r} Q84 -{r} 114 -93 L{r} -39 V40 L111 99 Q69 {r} 0 {r} Q-69 {r} -111 99 L-{r} 40 V-39 L-114 -93 Q-84 -{r} 0 -{r}Z"
        ),
        _ => format!("M0 -{r} A{r} {r} 0 1 1 0 {r} A{r} {r} 0 1 1 0 -{r}Z"),
    }
}

fn band(o: &mut String, m: Mark) {
    let r = m.inner() + 7;
    let ring = outline(m, r);
    stroke(o, &ring, "#21212a", 17.);
    stroke(o, &ring, METAL, 14.);
    stroke(o, &outline(m, r + 5), m.colors()[0], 2.0);
    if m.tier >= 3 {
        stroke(o, &outline(m, r - 4), EDGE, 3.0);
    }
    // A second gold/silver wire frames the insignia worn by the high leagues.
    if m.tier >= 7 {
        stroke(o, &outline(m, r + 13), EDGE, 6.0);
        stroke(o, &outline(m, r + 13), m.colors()[0], 2.0);
    }
}

fn clasp(o: &mut String, m: Mark) {
    let n = i32::from(m.step);
    if n == 0 {
        return;
    }
    let baseline = m.inner() + 6;
    let w = 17 * n + 7;
    write!(o, r#"<g transform="translate(0 {baseline})">"#).unwrap();
    flip(o, |o| {
        if n >= 2 {
            path(
                o,
                &format!(
                    "M{} -3 Q{} -14 {} -24 L{} -8 L{} -12 Q{} 5 {} 9 Z",
                    w - 9,
                    w + 7,
                    w + 20,
                    w + 13,
                    w + 26,
                    w + 19,
                    w - 1
                ),
                METAL,
            );
        }
        if n >= 3 {
            path(
                o,
                &format!(
                    "M{} 6 Q{} -2 {} -9 L{} 5 L{} 3 L{} 17 Z",
                    w - 11,
                    w + 9,
                    w + 29,
                    w + 20,
                    w + 26,
                    w + 7
                ),
                EDGE,
            );
        }
    });
    path(
        o,
        &format!(
            "M-{} -9 Q0 -2 {} -9 L{} 4 L{} 18 L0 {} L-{} 18 L-{} 4 Z",
            w - 3,
            w - 3,
            w + 3,
            w - 7,
            20 + n * 2,
            w - 7,
            w + 3
        ),
        METAL,
    );
    stroke(
        o,
        &format!("M-{} 0 Q0 8 {} 0", w - 5, w - 5),
        m.colors()[2],
        2.2,
    );
    mounted_stone(o, m, n, w);
    o.push_str("</g>");
}

// Every division bears one stone. Its cut and mounting become more refined.
fn mounted_stone(o: &mut String, m: Mark, n: i32, w: i32) {
    let width = 10 + n * 5;
    let height = 9 + n * 3;
    if n >= 2 {
        let rim = width + 6;
        path(
            o,
            &format!(
                "M-{rim} 10 L-{} -3 H{} L{rim} 10 L0 {}Z",
                width - 2,
                width - 2,
                16 + height
            ),
            EDGE,
        );
    }
    ornament::stone(
        o,
        0,
        10,
        width,
        height,
        if m.tier < 3 { METAL } else { GEM },
        m.colors()[0],
    );
    if n >= 3 {
        flip(o, |o| {
            stroke(
                o,
                &format!("M{} 9 Q{} 17 {} 5", width + 3, width + 16, w - 2),
                m.colors()[0],
                2.5,
            );
        });
    }
    if n == 4 {
        stroke(
            o,
            "M-22 8 L-9 9 L0 -6 L9 9 L22 8 M-9 9 L0 27 L9 9",
            m.colors()[0],
            1.6,
        );
    }
}

fn image(m: Mark) -> Result<image::RgbaImage> {
    let mut body = defs(m);
    let cut = if m.deck {
        octagon(m.inner() - 3, 25)
    } else {
        let r = m.inner() - 3;
        format!("M0 -{r} A{r} {r} 0 1 1 0 {r} A{r} {r} 0 1 1 0 -{r}Z")
    };
    write!(
        body,
        r#"<defs><mask id="clear" x="-192" y="-192" width="384" height="384" maskUnits="userSpaceOnUse"><rect x="-192" y="-192" width="384" height="384" fill="white"/><path d="{cut}" fill="black"/></mask></defs><g transform="translate(192 192)"><g mask="url(#clear)">"#
    )?;
    if m.deck {
        // The exposed lower-left and upper-right leaves of a stack of bound cards.
        for (dx, dy) in [(-6, 5), (6, -4)] {
            write!(body, r#"<g transform="translate({dx} {dy})">"#)?;
            stroke(&mut body, &outline(m, m.inner() + 5), EDGE, 7.);
            body.push_str("</g>");
        }
    }
    ornament::back(&mut body, m);
    band(&mut body, m);
    ornament::front(&mut body, m);
    clasp(&mut body, m);
    body.push_str("</g></g>");
    render(&usvg::Options::default(), 384, 384, &body)
}

pub(super) fn write(root: &Path) -> Result<()> {
    for deck in [false, true] {
        let dir = root
            .join("art/baylee/ranks/frames")
            .join(if deck { "deck" } else { "player" });
        fs::create_dir_all(&dir)?;
        for (tier, (name, divisions)) in LEAGUES.into_iter().enumerate() {
            for step in 0..=divisions {
                let suffix = if step == 0 {
                    ""
                } else {
                    ["", "-i", "-ii", "-iii", "-iv"][usize::from(1 + divisions - step)]
                };
                image(Mark {
                    tier,
                    deck,
                    step,
                    place: 0,
                })?
                .save(dir.join(format!("{name}{suffix}.png")))?;
            }
            if tier == 10 {
                for place in 1..=3 {
                    image(Mark {
                        tier,
                        deck,
                        step: 0,
                        place,
                    })?
                    .save(dir.join(format!("champion-{place}.png")))?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_division_mounts_one_central_stone_in_a_progressively_larger_setting() {
        for deck in [false, true] {
            for (tier, &(_, divisions)) in LEAGUES.iter().enumerate() {
                let mut previous = 0;
                for step in 1..=divisions {
                    let mark = Mark {
                        tier,
                        deck,
                        step,
                        place: 0,
                    };
                    let mut engraving = String::new();
                    clasp(&mut engraving, mark);
                    // One facet group from stone(), centered on the same axis at every step.
                    assert_eq!(engraving.matches(") scale(").count(), 1);
                    assert!(engraving.contains("translate(0 10) scale("));
                    let body = format!(
                        r#"{}<g transform="translate(192 192)">{engraving}</g>"#,
                        defs(mark)
                    );
                    let pixels = render(&usvg::Options::default(), 384, 384, &body).unwrap();
                    let area = pixels.pixels().filter(|p| p[3] > 127).count();
                    assert!(area > previous, "tier {tier}, step {step}, deck {deck}");
                    previous = area;
                }
            }
        }
    }
}
