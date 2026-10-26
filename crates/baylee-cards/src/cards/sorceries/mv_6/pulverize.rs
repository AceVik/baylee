//! Pulverize — {4}{R}{R} — Sorcery
//! Oracle: You may sacrifice two Mountains rather than pay this spell's mana cost.
//! Oracle: Destroy all artifacts.
//! Set: MMQ #207 — Mercadian Masques | Scryfall ID: afbbc44d-60fb-45fc-a588-14aab0340134 | Oracle ID: d403d1f3-5a92-4f4c-aff2-cff6a0b3b1ec
// NOT SUPPORTED: optional alternative cost (sacrificing two Mountains rather than paying mana cost) not supported
// PARTIAL — omits optional alternative cost (sacrificing two Mountains rather than paying mana cost).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PULVERIZE,
    oracle_id = "d403d1f3-5a92-4f4c-aff2-cff6a0b3b1ec",
    scryfall_id = "afbbc44d-60fb-45fc-a588-14aab0340134",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "optional alternative cost (sacrificing two Mountains rather than paying mana cost) not supported"
    ),
    faces = &[face!(
        name = "Pulverize",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::DestroyAll {
        filter: &Filter::ARTIFACT,
        no_regen: false,
    }]),],
);
