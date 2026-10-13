//! The keyword border's arithmetic (commit 5f6fda25f): which keywords make
//! which layers, what the table and the preview are told about them, how
//! far a wider border reaches and when it stands, and the order the shader
//! lays the layers in.

use super::flying_tests::creature;
use super::*;
use crate::cardmat::tests::wgsl_const;
use crate::shellmat::{
    self, Footprint, MAX_LAYERS, RIM_DROP, RIM_RISE, RING_INNER, RING_LAYER, ShellKind, ShellLook,
    ShellParams, Shells, Ward,
};
use crate::shellui::{PREVIEW_QUAD, ShellUiParams};
use baylee_client_core::board::CardGroup;

/// `shell_common.wesl`, as the shader modules read it.
const COMMON: &str = include_str!("../shaders/shell_common.wesl");

/// Whether two lengths are the same to a hair.
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-6
}

/// A creature wearing exactly `badges`.
fn wearing(badges: &[KeywordBadge]) -> CardGroup {
    creature(1, badges.to_vec())
}

/// Shroud swallows hexproof: a permanent with both wears one layer, and
/// every other combination keeps all its bits.
#[test]
fn shroud_swallows_hexproof_and_nothing_else() {
    let (i, d, h, s) = (
        Ward::INDESTRUCTIBLE,
        Ward::DEFENDER,
        Ward::HEXPROOF,
        Ward::SHROUD,
    );
    assert_eq!(Ward::of(h | s), Ward(s), "hexproof and shroud are shroud");
    assert_eq!(Ward::of(h | s).layers(), 1);
    assert_eq!(Ward::of(i | d | h | s), Ward(i | d | s), "all four");
    assert_eq!(Ward::of(i | d | h | s).layers(), 3);
    for bits in [
        0,
        i,
        d,
        h,
        s,
        i | d,
        i | h,
        d | h,
        i | d | h,
        i | s,
        d | s,
        i | d | s,
    ] {
        assert_eq!(Ward::of(bits), Ward(bits), "{bits:#06b} is kept as it is");
    }
}

/// A border has as many layers as keywords it says, and is empty only when
/// it says none.
#[test]
fn a_ward_has_a_layer_for_each_keyword_and_is_empty_only_without() {
    assert_eq!(Ward::NONE.layers(), 0);
    assert!(Ward::NONE.is_empty());
    assert!(Ward::default().is_empty());
    for (bits, layers) in [
        (Ward::INDESTRUCTIBLE, 1),
        (Ward::DEFENDER | Ward::SHROUD, 2),
        (Ward::INDESTRUCTIBLE | Ward::DEFENDER | Ward::HEXPROOF, 3),
        (0b1111, 4),
    ] {
        assert_eq!(Ward(bits).layers(), layers, "{bits:#06b}");
        assert!(!Ward(bits).is_empty(), "{bits:#06b}");
    }
}

/// The four keyword bits are the four values the shader names, one bit each
/// and all different.
#[test]
fn the_four_keywords_are_four_distinct_bits() {
    let all = [
        Ward::INDESTRUCTIBLE,
        Ward::DEFENDER,
        Ward::HEXPROOF,
        Ward::SHROUD,
    ];
    assert_eq!(all, [1, 2, 4, 8]);
    assert_eq!(all.iter().fold(0, |acc, bit| acc | bit), 0b1111);
}

/// `Shells::of` reads each of the four keyword badges into its bit, ignores
/// the badges that are no keyword of a border, and applies the shroud rule.
#[test]
fn shells_read_the_keyword_badges_into_the_ward() {
    for (badge, bit) in [
        (KeywordBadge::Indestructible, Ward::INDESTRUCTIBLE),
        (KeywordBadge::Defender, Ward::DEFENDER),
        (KeywordBadge::Hexproof, Ward::HEXPROOF),
        (KeywordBadge::Shroud, Ward::SHROUD),
    ] {
        assert_eq!(Shells::of(&wearing(&[badge])).ward, Ward(bit), "{badge:?}");
    }
    assert_eq!(Shells::of(&wearing(&[])).ward, Ward::NONE, "no badge");
    assert_eq!(
        Shells::of(&wearing(&[KeywordBadge::Flying])).ward,
        Ward::NONE,
        "flying makes no border"
    );
    let both = Shells::of(&wearing(&[KeywordBadge::Hexproof, KeywordBadge::Shroud]));
    assert_eq!(both.ward, Ward(Ward::SHROUD), "shroud swallows hexproof");
    let all = Shells::of(&wearing(&[
        KeywordBadge::Shroud,
        KeywordBadge::Hexproof,
        KeywordBadge::Defender,
        KeywordBadge::Indestructible,
    ]));
    assert_eq!(all.ward.layers(), 3, "the order of the badges is no matter");
}

/// Summoning sickness is the wave and nothing else of a group's shells.
#[test]
fn summoning_sickness_is_the_wave() {
    let mut group = wearing(&[]);
    assert!(!Shells::of(&group).wave);
    group.summoning_sick = true;
    let shells = Shells::of(&group);
    assert!(shells.wave);
    assert!(shells.ward.is_empty(), "a wave says no keyword");
}

/// Only a border reaches past the card's edge: a wave alone reaches
/// nothing, a ward of any size does.
#[test]
fn only_a_border_reaches_off_the_card() {
    assert!(!Shells::default().reach_off_the_card());
    let wave = Shells {
        ward: Ward::NONE,
        wave: true,
    };
    assert!(!wave.reach_off_the_card(), "a wave alone");
    for ward in [Ward(Ward::SHROUD), Ward(0b1111)] {
        for wave in [false, true] {
            assert!(Shells { ward, wave }.reach_off_the_card(), "{ward:?}");
        }
    }
}

/// The preview paints the wave first and the border over it, and each only
/// when there is one.
#[test]
fn the_preview_stands_the_wave_then_the_border() {
    let ward = Ward(Ward::DEFENDER);
    assert_eq!(Shells::default().standing(), []);
    assert_eq!(
        Shells { ward, wave: true }.standing(),
        [ShellLook::wave(), ShellLook::border(ward)]
    );
    assert_eq!(
        Shells { ward, wave: false }.standing(),
        [ShellLook::border(ward)]
    );
    assert_eq!(
        Shells {
            ward: Ward::NONE,
            wave: true
        }
        .standing(),
        [ShellLook::wave()]
    );
}

/// The border's foot is `0.02 + 0.03 × layers` out, three layers at most
/// and one at least, so a wider border is a wider band.
#[test]
fn a_border_is_three_hundredths_wider_for_each_layer() {
    assert!(near(shellmat::border_margin(1), 0.05));
    assert!(near(shellmat::border_margin(2), 0.08));
    assert!(near(shellmat::border_margin(3), 0.11));
    assert!(near(shellmat::border_margin(0), shellmat::border_margin(1)));
    assert!(near(
        shellmat::border_margin(MAX_LAYERS + 5),
        shellmat::border_margin(MAX_LAYERS)
    ));
}

/// A standing border's two edges are its top at the card's edge, a rim's
/// rise over the face, and its foot one margin out and a rim's drop down.
#[test]
fn a_border_runs_from_its_rim_to_the_felt() {
    for layers in 1..=MAX_LAYERS {
        let [top, foot] = shellmat::border_edges(layers);
        assert_eq!(top, (0.0, RIM_RISE), "{layers} layers");
        assert!(near(foot.0, shellmat::border_margin(layers)), "{layers}");
        assert!(near(foot.1, -RIM_DROP), "{layers}");
    }
}

/// The ring a border lies down to starts at the ring's inner distance and
/// grows by a ring layer for each layer, flat on the felt.
#[test]
fn a_ring_grows_a_ring_layer_for_each_layer() {
    for layers in 1..=MAX_LAYERS {
        #[expect(clippy::cast_precision_loss)] // three at most
        let n = layers as f32;
        let [(inner, flat_in), (outer, flat_out)] = shellmat::ring_edges(layers);
        assert!(near(inner, RING_INNER), "{layers}");
        assert!(near(outer, RING_INNER + RING_LAYER * n), "{layers}");
        assert!(flat_in.abs() < f32::EPSILON && flat_out.abs() < f32::EPSILON);
    }
}

/// The table's uniform: the look's kind, keywords and layers, a ring's band
/// from the ring's edges and a border's or a wave's from nothing to the
/// border's margin.
#[test]
fn the_table_is_told_the_look_and_its_band() {
    let ward = Ward(Ward::INDESTRUCTIBLE | Ward::SHROUD);
    let motion = 0.5;

    let ring = ShellParams::new(ShellLook::ring(ward), motion);
    let [(inner, _), (outer, _)] = shellmat::ring_edges(2);
    assert_eq!(ring.kind, ShellKind::Ring as u32);
    assert_eq!((ring.ward, ring.layers), (ward.0, 2));
    assert!(
        near(ring.inner, inner) && near(ring.outer, outer),
        "{ring:?}"
    );
    assert!(near(ring.motion, motion));

    let border = ShellParams::new(ShellLook::border(ward), motion);
    assert_eq!(border.kind, ShellKind::Border as u32);
    assert_eq!((border.ward, border.layers), (ward.0, 2));
    assert!(border.inner.abs() < f32::EPSILON, "{border:?}");
    assert!(near(border.outer, shellmat::border_margin(2)), "{border:?}");

    let wave = ShellParams::new(ShellLook::wave(), motion);
    assert_eq!(wave.kind, ShellKind::Wave as u32);
    assert_eq!((wave.ward, wave.layers), (0, 0));
    assert!(wave.inner.abs() < f32::EPSILON, "{wave:?}");
    assert!(near(wave.outer, shellmat::border_margin(0)), "{wave:?}");
}

/// The preview is told what the table is: the same keywords and layers, the
/// border's margin, and the quad of the preview.
#[test]
fn the_preview_is_told_the_same_border() {
    let ward = Ward(Ward::DEFENDER | Ward::HEXPROOF | Ward::SHROUD);
    let on = ShellUiParams::new(ShellLook::border(ward), 1.0);
    assert_eq!(on.kind, ShellKind::Border as u32);
    assert_eq!((on.ward, on.layers), (ward.0, ward.layers()));
    assert!(near(on.margin, shellmat::border_margin(ward.layers())));
    assert_eq!(on.quad.to_array(), PREVIEW_QUAD);
    assert!(near(on.motion, 1.0));

    let wave = ShellUiParams::new(ShellLook::wave(), 0.0);
    assert_eq!(wave.kind, ShellKind::Wave as u32);
    assert_eq!((wave.ward, wave.layers), (0, 0));
    assert!(near(wave.margin, shellmat::border_margin(0)));
    assert_eq!(wave.quad.to_array(), PREVIEW_QUAD);
}

/// A wider border never stands where a narrower one does not: over a sweep
/// of a neighbour sliding away along the row, three layers standing means
/// one layer stands, and somewhere the two part, one standing and three not.
#[test]
fn a_wider_border_never_stands_where_a_narrower_one_does_not() {
    let slot = *TableLayout::new(&[PlayerId::new(0)], 16.0 / 9.0, None)
        .slot(PlayerId::new(0))
        .expect("a one-seat table seats its one player");
    let eye = Vec3::new(0.0, 20.0, 8.0);
    let me = Footprint::of(&card_transform(&slot, Vec2::ZERO, false, 0.004));
    let mut parted = 0;
    let mut stood = 0;
    for step in 0..=40 {
        #[expect(clippy::cast_precision_loss)] // forty at most
        let x = 1.0 + step as f32 * 0.005;
        for lift in [0.0, 0.004] {
            for headroom in [0.0, shellmat::STAND_AGAIN] {
                let beside =
                    || Footprint::of(&card_transform(&slot, Vec2::new(x, 0.0), false, lift));
                let narrow = shellmat::border_stands(1, &me, [beside()], eye, headroom);
                let wide = shellmat::border_stands(3, &me, [beside()], eye, headroom);
                assert!(
                    !wide || narrow,
                    "three layers stand and one does not: x {x}, lift {lift}, headroom {headroom}"
                );
                stood += usize::from(wide);
                parted += usize::from(narrow && !wide);
            }
        }
    }
    assert!(
        stood > 0,
        "three layers never stood: the sweep is too tight"
    );
    assert!(
        parted > 0,
        "no offset parts one layer from three: the sweep tests nothing"
    );
}

/// The shader lays the layers inside out over the four keyword bits, in
/// increasing order of value: the loop runs over four bits, and the four
/// `WARD_*` constants are declared with values that increase.
#[test]
fn the_shader_lays_the_layers_in_the_order_of_the_bits() {
    let open = COMMON.find("fn ward_style(").expect("the layer lookup");
    let body = &COMMON[open..open + COMMON[open..].find("\n}").expect("it closes")];
    assert!(
        body.contains("bit < 4u"),
        "the loop covers four bits: {body}"
    );
    assert!(
        body.contains("1u << bit"),
        "each bit is its own style: {body}"
    );

    let names = [
        "WARD_INDESTRUCTIBLE",
        "WARD_DEFENDER",
        "WARD_HEXPROOF",
        "WARD_SHROUD",
    ];
    let mut last_at = 0;
    let mut last_value = 0.0;
    for name in names {
        let at = COMMON
            .find(&format!("const {name}:"))
            .unwrap_or_else(|| panic!("no `{name}` in the shader"));
        let value = wgsl_const(COMMON, name);
        assert!(at > last_at, "{name} is declared out of order");
        assert!(value > last_value, "{name} does not increase");
        last_at = at;
        last_value = value;
    }
    assert!(
        (last_value - 8.0).abs() < f32::EPSILON,
        "four bits make the highest 8"
    );
}
