//! The silhouette of each order, carved in front view about (0, 0).
use super::{EDGE, GEM, IVORY, METAL, Mark, dot, flip, path, stroke, turn};
use std::fmt::Write;

#[allow(clippy::many_single_char_names)] // Coordinates and extents in the engraving plane.
pub(super) fn stone(o: &mut String, x: i32, y: i32, w: i32, h: i32, fill: &str, light: &str) {
    write!(
        o,
        r#"<g transform="translate({x} {y}) scale({} {})">"#,
        f64::from(w) / 20.,
        f64::from(h) / 20.
    )
    .unwrap();
    path(o, "M0 -21 L17 -5 L19 3 L0 23 L-19 3 L-17 -5Z", METAL);
    path(o, "M0 -16 L13 -3 L14 2 L0 17 L-14 2 L-13 -3Z", fill);
    write!(o,r##"<path d="M0 -15 L-5 -1 L-13 2Z M13 1 L4 -2 L0 15Z" fill="{light}" opacity=".78"/><path d="M-5 -1 L0 -6 L5 -1 L0 5Z" fill="#fff" opacity=".69"/><path d="M-5 -1 L0 15 L-12 3Z" fill="#243052" opacity=".32"/></g>"##).unwrap();
}

fn folded(o: &mut String, m: Mark) {
    if m.deck {
        flip(o, |o| {
            path(
                o,
                "M116 -162 L159 -162 L164 -122 L150 -130 L130 -148Z",
                METAL,
            );
            path(o, "M116 -162 L145 -157 L130 -148Z", "#fff0c7");
            path(
                o,
                "M154 106 L167 121 L162 153 L137 166 L122 151 L149 137Z",
                METAL,
            );
        });
    } else {
        flip(o, |o| {
            path(o, "M87 -126 L117 -150 L147 -126 L138 -95 L128 -103Z", METAL);
            path(o, "M117 -150 L126 -121 L147 -126Z", "#f6e7bd");
            path(o, "M86 136 L119 146 L140 96 L121 111Z", METAL);
        });
    }
}
fn timber(o: &mut String, m: Mark) {
    flip(o, |o| {
        if m.deck {
            path(
                o,
                "M97 -150 L149 -166 L169 -148 L154 -91 L145 -115 L151 -142 L128 -143 L104 -130Z",
                METAL,
            );
            path(
                o,
                "M138 95 L158 103 L165 141 L132 163 L100 148 L130 139Z",
                EDGE,
            );
            stroke(
                o,
                "M113 -145 Q137 -159 155 -149 Q156 -139 150 -119",
                m.colors()[0],
                3.,
            );
        } else {
            path(
                o,
                "M91 -121 Q135 -119 150 -158 Q157 -132 145 -117 Q162 -124 168 -139 Q171 -105 140 -83 L132 -58Z",
                METAL,
            );
            stroke(
                o,
                "M110 -109 Q133 -112 149 -142 M131 -84 Q153 -104 164 -121",
                m.colors()[2],
                3.,
            );
            path(
                o,
                "M110 107 Q132 118 143 102 Q142 130 112 137 L90 128Z",
                EDGE,
            );
        }
        for (x, y) in [(139, -72), (152, -45)] {
            stroke(
                o,
                &format!("M{} {} q10 -8 13 -17", x - 3, y),
                m.colors()[0],
                3.,
            );
        }
    });
}
fn iron(o: &mut String, m: Mark) {
    for a in if m.deck {
        [0, 90, 180, 270]
    } else {
        [45, 135, 225, 315]
    } {
        turn(o, a, |o| {
            path(
                o,
                "M-65 -143 L-53 -160 L-32 -166 L-23 -172 H23 L32 -166 L53 -160 L65 -143 L46 -130 H-46Z",
                EDGE,
            );
            path(o, "M-46 -143 L-35 -156 H35 L46 -143 L36 -135 H-36Z", METAL);
            for x in [-27, 27] {
                dot(o, x, -147, 7, "#43535e");
                stroke(o, &format!("M{} -150 l7 5", x - 3), "#dbe5ed", 2.);
            }
        });
    }
}
fn laurel(o: &mut String, m: Mark) {
    flip(o, |o| {
        if m.deck {
            o.push_str(r#"<g transform="translate(5 1) rotate(7 135 -75)">"#);
        }
        stroke(o, "M81 121 Q186 79 151 -77 Q145 -107 124 -134", EDGE, 9.);
        for (x, y, a) in [
            (117, 98, 55),
            (140, 64, 27),
            (151, 26, 9),
            (151, -16, -6),
            (143, -58, -23),
            (130, -96, -38),
        ] {
            write!(o, r#"<g transform="translate({x} {y}) rotate({a})">"#).unwrap();
            path(o, "M0 11 Q2 -12 25 -29 Q31 -5 5 13Z", METAL);
            stroke(o, "M5 6 L22 -21", m.colors()[2], 2.4);
            o.push_str("</g>");
        }
        if m.deck {
            o.push_str("</g>");
        }
    });
}
fn crescent(o: &mut String, m: Mark) {
    flip(o, |o| {
        if m.deck {
            path(
                o,
                "M102 -137 L172 -164 Q169 -117 151 -100 L154 -137 L111 -113Z",
                METAL,
            );
            path(o, "M116 144 Q157 111 163 77 Q182 140 131 157Z", METAL);
        } else {
            path(
                o,
                "M70 -126 Q102 -143 115 -170 Q134 -146 122 -119 Q161 -128 166 -153 Q178 -106 128 -80 L112 -99Z",
                METAL,
            );
            path(
                o,
                "M132 65 Q147 93 156 73 Q161 111 127 131 L102 116Z",
                METAL,
            );
        }
        stroke(o, "M111 -115 Q137 -114 155 -135", "#f4ffff", 3.);
    });
}
fn sun(o: &mut String, m: Mark) {
    for a in [-68i32, -48, -28, -10, 10, 28, 48, 68] {
        let r = if a.abs() == 28 || a.abs() == 68 {
            186
        } else {
            172
        };
        turn(o, a, |o| {
            let tail = if m.deck { 128 } else { 138 };
            path(
                o,
                &format!("M-10 -{tail} L-9 -151 L0 -{r} L10 -150 L12 -{tail}Z"),
                METAL,
            );
            path(o, &format!("M0 -{r} L8 -150 L0 -{tail}Z"), EDGE);
        });
    }
    if m.deck {
        flip(o, |o| {
            path(o, "M126 117 L164 118 L164 150 L149 165 L114 164Z", METAL);
        });
    }
}
fn deco(o: &mut String, m: Mark) {
    flip(o, |o| {
        if m.deck {
            o.push_str(r#"<g transform="translate(5 -2)">"#);
        }
        path(
            o,
            "M73 -117 L94 -141 L123 -143 L132 -162 L168 -163 L164 -125 L153 -111 L149 -84 L125 -61Z",
            EDGE,
        );
        path(
            o,
            "M78 -116 L108 -144 L125 -143 L140 -156 L153 -156 L150 -142 L130 -122 L115 -122 L99 -103Z",
            METAL,
        );
        path(
            o,
            "M111 -102 L140 -130 L157 -137 L153 -119 L136 -102 L130 -81Z",
            GEM,
        );
        path(
            o,
            "M111 101 L134 81 L148 94 L147 133 L134 146 L95 148 L85 134Z",
            METAL,
        );
        stroke(o, "M110 137 H129 L138 128 V110", m.colors()[3], 5.);
        if m.deck {
            o.push_str("</g>");
        }
    });
}
fn crystal(o: &mut String, m: Mark) {
    flip(o, |o| {
        if m.deck {
            o.push_str(r#"<g transform="translate(4 -2) rotate(-8 132 -80)">"#);
        }
        for (x, y, a, s) in [
            (94, -113, -11, 0.87),
            (117, -105, 7, 1.08),
            (138, -80, 27, 0.97),
            (146, -54, 45, 0.70),
        ] {
            write!(
                o,
                r#"<g transform="translate({x} {y}) rotate({a}) scale({s})">"#
            )
            .unwrap();
            path(
                o,
                "M-13 12 L-18 -16 L-10 -47 L2 -65 L14 -48 L20 -15 L11 11Z",
                GEM,
            );
            path(o, "M2 -64 L5 -19 L11 11 L20 -15 L14 -48Z", "#468dcc");
            path(o, "M2 -63 L-9 -19 L-17 -14 L-9 -46Z", "#c8ffff");
            stroke(o, "M-9 -18 L5 -19 L14 -47", "#c6eafa", 2.);
            o.push_str("</g>");
        }
        if m.deck {
            o.push_str("</g>");
        }
        stone(o, 113, 102, 15, 24, GEM, m.colors()[0]);
    });
}

// Overlapping tapered vanes with a chased spine and a bright return edge.
// They attach behind the bezel, leaving genuine gaps between the feather tips.
#[allow(clippy::many_single_char_names)] // Parametric feather curve.
fn wings(o: &mut String, m: Mark, count: i32, length: f64) {
    flip(o, |o| {
        let angle = if m.deck { -8 } else { 7 };
        write!(o, r#"<g transform="rotate({angle} 110 -67)">"#).unwrap();
        for i in (0..count).rev() {
            let t = f64::from(i);
            let x = 82.0 + t * 9.2;
            let y = -94.0 + t * 12.;
            let end_x = (140. + length * 32. + t * 2.).min(183.);
            let end_y = -164. + t * 19. - length * 11.;
            let mid_x = f64::midpoint(end_x, x);
            let mid_y = f64::midpoint(end_y, y);
            let fill = if m.tier == 8 {
                if i % 2 == 0 { GEM } else { METAL }
            } else if m.tier == 9 {
                if i % 2 == 0 { METAL } else { GEM }
            } else if i % 2 == 0 {
                IVORY
            } else {
                METAL
            };
            let d = format!(
                "M{x} {y} Q{mid_x} {} {end_x} {end_y} Q{} {} {} {} Q{} {} {} {}Z",
                y + 2.,
                end_x - 2.,
                mid_y + 10.,
                x + 21.,
                y + 17.,
                x + 11.,
                y + 19.,
                x - 2.,
                y + 12.
            );
            path(o, &d, fill);
            stroke(
                o,
                &format!(
                    "M{} {} Q{} {} {} {}",
                    x + 8.,
                    y + 10.,
                    mid_x + 4.,
                    mid_y + 13.,
                    end_x - 5.,
                    end_y + 7.
                ),
                m.colors()[2],
                2.4,
            );
        }
        o.push_str("</g>");
    });
}
fn horns(o: &mut String, m: Mark) {
    flip(o, |o| {
        let a = if m.deck { -7 } else { 3 };
        write!(o, r#"<g transform="rotate({a} 90 -100)">"#).unwrap();
        path(
            o,
            "M51 -119 Q99 -143 106 -179 Q125 -157 116 -134 Q137 -151 142 -178 Q158 -120 102 -92Z",
            METAL,
        );
        path(o, "M98 -117 Q121 -129 137 -155 Q132 -126 109 -107Z", GEM);
        stroke(o, "M75 -123 Q101 -141 106 -164", m.colors()[0], 3.);
        o.push_str("</g>");
    });
}

pub(super) fn back(o: &mut String, m: Mark) {
    match m.tier {
        0 => folded(o, m),
        1 => timber(o, m),
        2 => iron(o, m),
        3 => laurel(o, m),
        4 => crescent(o, m),
        5 => sun(o, m),
        6 => deco(o, m),
        7 => crystal(o, m),
        8 => wings(o, m, 3, 0.32),
        9 => {
            wings(o, m, 4, 0.52);
            horns(o, m);
        }
        _ => {
            if m.place == 1 {
                sun(o, m);
            }
            wings(
                o,
                m,
                match m.place {
                    0 => 4,
                    3 => 5,
                    _ => 6,
                },
                match m.place {
                    0 => 0.55,
                    3 => 0.72,
                    2 => 0.92,
                    _ => 1.13,
                },
            );
            if m.place == 3 {
                laurel(o, m);
            } else if matches!(m.place, 1 | 2) {
                super::podium::gallery(o, m);
                if m.place == 1 {
                    horns(o, m);
                }
            }
        }
    }
}

fn crest(o: &mut String, m: Mark) {
    let gold = METAL;
    let gleam = m.colors()[0];
    // A mantle crossing the top and lower returns visually join both wings.
    flip(o, |o| {
        path(
            o,
            "M8 -135 Q42 -149 71 -140 Q92 -137 112 -148 Q104 -124 72 -119 L62 -125 Q34 -120 17 -109Z",
            gold,
        );
        stroke(o, "M20 -130 Q47 -143 72 -134 Q86 -131 103 -140", gleam, 3.);
        path(
            o,
            "M52 116 Q93 112 112 92 Q123 108 107 127 Q125 124 137 113 Q128 147 85 150 L63 137 L38 141Z",
            gold,
        );
        stroke(o, "M68 130 Q96 126 110 111", gleam, 3.);
    });
    let center = if m.deck { -137 } else { -139 };
    match m.tier {
        8 => {
            path(
                o,
                "M-34 -130 L-44 -150 L-18 -145 L0 -180 L17 -146 L42 -154 L34 -128 L0 -113Z",
                gold,
            );
            stone(o, 0, center, 18, 31, GEM, gleam);
            stone(o, 0, 137, 16, 24, GEM, gleam);
        }
        9 => {
            path(
                o,
                "M-61 -129 L-62 -151 L-37 -144 L-34 -171 L-12 -155 L0 -183 L13 -155 L36 -173 L39 -144 L62 -152 L62 -128 L0 -109Z",
                gold,
            );
            stone(o, 0, center - 3, 24, 32, GEM, gleam);
            flip(o, |o| stone(o, 37, -142, 11, 17, GEM, gleam));
            path(
                o,
                "M-51 127L-32 152 L-17 149 L0 179 L18 148 L32 155 L54 126 L0 117Z",
                gold,
            );
            stone(o, 0, 145, 15, 27, GEM, gleam);
        }
        _ => super::podium::crown(o, m, center),
    }
}

#[allow(clippy::many_single_char_names)] // Engraving coordinates.
pub(super) fn front(o: &mut String, m: Mark) {
    let r = m.inner() + 8;
    let light = m.colors()[0];
    if m.tier >= 8 {
        crest(o, m);
    } else if m.tier >= 3 {
        let style = if m.tier == 5 { METAL } else { GEM };
        let (w, h) = match m.tier {
            3 => (10, 10),
            4 => (14, 14),
            5 => (16, 22),
            6 => (21, 14),
            _ => (21, 31),
        };
        if m.tier == 4 {
            dot(o, 0, -r, 12, IVORY);
        } else {
            stone(o, 0, -r, w, h, style, light);
        }
        if m.step == 0 {
            stone(o, 0, r, 10, 12, style, light);
        }
    }
    if m.tier >= 8 {
        for angle in [-114, -90, -65, 65, 90, 114] {
            turn(o, angle, |o| {
                dot(o, 0, -r, 5, if m.tier == 8 { GEM } else { IVORY });
            });
        }
    }
    if m.place > 0 {
        // Twisted cable for rose laurels, bright beaded ribs for the silver wings,
        // and a second golden rail studded with opals for the crown.
        let amount = if m.place == 3 {
            22
        } else if m.place == 2 {
            30
        } else {
            36
        };
        for i in 0..amount {
            let angle = 360 * i / amount;
            let rr = r + 9;
            turn(o, angle, |o| {
                if m.place == 3 {
                    stroke(
                        o,
                        &format!("M-6 -{} Q0 -{} 6 -{}", rr - 4, rr + 4, rr + 1),
                        METAL,
                        5.,
                    );
                } else {
                    dot(o, 0, -rr, if m.place == 1 { 4 } else { 3 }, IVORY);
                }
            });
        }
    }
    // The deck series uses stepped corner escutcheons and tiny exposed folio
    // edges; its title setting rests on a fan of three book leaves.
    if m.deck {
        for a in [45, 135, 225, 315] {
            turn(o, a, |o| {
                let rail = match m.tier {
                    0..=1 => 174,
                    2..=4 => 172,
                    5..=7 => 164,
                    _ => 149,
                };
                path(
                    o,
                    &format!(
                        "M-20 -{} L-15 -{} L0 -{} L15 -{} L20 -{} L0 -{}Z",
                        rail - 8,
                        rail + 1,
                        rail + 5,
                        rail + 1,
                        rail - 8,
                        rail - 15
                    ),
                    if m.tier >= 5 { METAL } else { EDGE },
                );
            });
        }
        if m.tier >= 6 {
            flip(o, |o| {
                stroke(o, "M88 -132 L120 -132 L136 -116", light, 3.);
            });
        }
    }
}
