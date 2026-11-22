//! Jewellery of the four Champion mounts.
use super::ornament::stone;
use super::{GEM, IVORY, METAL, Mark, dot, flip, path, stroke};
use std::fmt::Write;
// The silver order is mounted in paired swan-neck brooches. The gold order
// has a deeper three-arched gallery, with a cabochon in each pierced bay.
pub(super) fn gallery(o: &mut String, m: Mark) {
    let first = m.place == 1;
    flip(o, |o| {
        let outer = if first { 182 } else { 174 };
        path(
            o,
            &format!(
                "M119 -103 Q159 -83 {outer} -72 Q157 -65 156 -41 Q180 -53 {outer} -58 Q174 -29 153 -9 Q167 -14 {outer} -21 Q170 14 156 37 Q174 30 {outer} 17 Q175 63 143 82 Q161 75 171 59 Q166 101 120 121 L98 114 Q137 78 135 48 Q150 6 131 -74Z"
            ),
            METAL,
        );
        let scroll = "M124 -89 Q174 -60 151 -36 Q137 -23 149 -18 Q166 -17 176 -45 M153 -12 Q174 -27 164 1 Q159 18 148 31 Q136 53 148 54 Q159 55 169 40 M145 66 Q161 60 165 53 Q157 99 119 109";
        stroke(o, scroll, "#4a4d64", 6.);
        stroke(o, scroll, if first { "#fff0a4" } else { "#effbff" }, 3.);
        for (x, y, a) in [(149, -59, -21), (153, 10, 13), (139, 72, 32)] {
            write!(o, r#"<g transform="translate({x} {y}) rotate({a})">"#).unwrap();
            stone(
                o,
                0,
                0,
                if first { 13 } else { 11 },
                if first { 20 } else { 17 },
                if first { IVORY } else { GEM },
                m.colors()[0],
            );
            o.push_str("</g>");
        }
        if first {
            for (x, y, a) in [(163, -82, -20), (168, -23, 0), (169, 34, 17), (145, 99, 34)] {
                write!(o, r#"<g transform="translate({x} {y}) rotate({a})">"#).unwrap();
                path(o, "M0 -16 Q14 -5 9 7 L0 16 Q-9 8 -9 -1Z", IVORY);
                path(o, "M0 -10 L5 1 L0 9 L-4 0Z", "#349aaf");
                o.push_str("</g>");
            }
            path(
                o,
                "M73 128 Q116 121 143 104 Q133 131 106 143 Q128 139 141 128 Q125 153 86 159 L58 139Z",
                METAL,
            );
            stroke(o, "M82 142 Q111 137 125 126", "#fff3c4", 3.);
            stone(o, 81, 141, 10, 14, GEM, m.colors()[0]);
        }
    });
}
pub(super) fn crown(o: &mut String, m: Mark, center: i32) {
    let gold = METAL;
    let deep = m.colors()[2];
    let gleam = m.colors()[0];
    let extra = match m.place {
        0 => 0,
        3 => 5,
        2 => 10,
        _ => 15,
    };
    let draw = |o: &mut String| {
        path(
            o,
            &format!(
                "M-84 -125 L-74 -150 L-51 -145 L-53 -{} L-25 -149 L-18 -{} L0 -159 L17 -{} L24 -149 L52 -{} L51 -145 L74 -150 L84 -125 Q40 -116 0 -108 Q-36 -115 -84 -125Z",
                156 + extra,
                163 + extra,
                163 + extra,
                156 + extra
            ),
            gold,
        );
        stroke(o, "M-73 -132 Q-34 -125 0 -120 Q37 -125 71 -132", deep, 4.);
        flip(o, |o| {
            stone(o, 52, -140, 9 + extra / 3, 12 + extra / 3, GEM, gleam);
        });
    };
    draw(o);
    match m.place {
        0 => {
            stone(o, 0, center - 6, 22, 26, IVORY, gleam);
            stone(o, 0, 136, 16, 22, IVORY, gleam);
        }
        3 => {
            flip(o, |o| {
                for (x, y, a) in [(27, -155, -35), (42, -159, -15), (58, -153, 20)] {
                    write!(o, r#"<g transform="translate({x} {y}) rotate({a})">"#).unwrap();
                    path(o, "M0 7 Q-11 -4 -2 -18 Q11 -10 0 7Z", METAL);
                    o.push_str("</g>");
                }
                path(o, "M17 128 Q45 129 65 148 Q49 155 26 145 L9 153Z", METAL);
            });
            stone(o, 0, center - 7, 28, 35, GEM, gleam);
            stone(o, 0, 145, 21, 26, GEM, gleam);
        }
        2 => {
            // Broad stepped sapphire cut with three pale side baguettes on either hand.
            path(
                o,
                "M-33 -161 L-18 -176 H18 L33 -161 V-139 L18 -124 H-18 L-33 -139Z",
                METAL,
            );
            path(
                o,
                "M-25 -158 L-14 -169 H14 L25 -158 V-143 L14 -132 H-14 L-25 -143Z",
                GEM,
            );
            stroke(
                o,
                "M-18 -156 L-11 -163 H11 L18 -156 V-146 L11 -139 H-11 L-18 -146Z",
                "#cce5ff",
                3.,
            );
            flip(o, |o| {
                for i in 0..3 {
                    stone(o, 40 + 17 * i, -151 + 3 * i, 8, 15, IVORY, gleam);
                }
                path(
                    o,
                    "M15 127 L61 127 L87 149 L57 147 L66 159 L25 149 Z",
                    IVORY,
                );
            });
            stone(o, 0, 145, 25, 30, GEM, gleam);
        }
        _ => brilliant(o, gleam),
    }
}
fn brilliant(o: &mut String, gleam: &str) {
    let gold = METAL;
    // The coronet has a raised central brilliant, pointed finials, gold arches
    // and a pavé row. None of these extra silhouettes occurs on #2 or #3.
    path(
        o,
        "M-69 -144 L-77 -174 L-51 -155 L-47 -181 L-22 -160 L0 -190 L21 -160 L47 -181 L51 -155 L78 -174 L68 -144 Q0 -119 -69 -144Z",
        gold,
    );
    flip(o, |o| {
        stroke(
            o,
            "M25 -145 Q31 -170 52 -176 Q63 -179 73 -169",
            "#fff0b1",
            4.,
        );
        for i in 0..5 {
            dot(o, 31 + 8 * i, -141 - i, 3, IVORY);
        }
        stone(o, 49, -165, 6, 9, GEM, gleam);
        path(
            o,
            "M7 126 Q41 125 75 139 L106 154 L74 152 L82 164 L41 151 L26 164Z",
            IVORY,
        );
    });
    path(o, "M-46 -155 L-23 -180 H23 L46 -155 L0 -116Z", gold);
    path(o, "M-40 -155 L-21 -174 H21 L40 -155 L0 -122Z", "#267fa1");
    path(o, "M-38 -155 L-10 -155 L0 -123Z", "#68d5fa");
    path(o, "M39 -155 L10 -155 L0 -123Z", "#aa8bc0");
    path(o, "M-10 -155 L0 -173 L10 -155 L0 -124Z", "#d3f8ff");
    stroke(o, "M-38 -155 H38", "#ffffff", 3.);
    stone(o, 0, 146, 28, 36, IVORY, gleam);
}
