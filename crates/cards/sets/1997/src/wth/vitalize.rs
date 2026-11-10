//! Vitalize — {G} — Instant
//! Oracle: Untap all creatures you control.
//! Set: 6ED #265 — Classic Sixth Edition | Scryfall ID: 435a4569-fa62-4e1a-b837-f68159cb270d | Oracle ID: cd3226c6-bd47-4246-8ab3-01316c5d1809
// IMPLEMENTED — Untaps all creatures you control.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VITALIZE,
    oracle_id = "cd3226c6-bd47-4246-8ab3-01316c5d1809",
    scryfall_id = "435a4569-fa62-4e1a-b837-f68159cb270d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vitalize",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::UntapAll {
        filter: &Filter::YOUR_CREATURE
    }]),],
);
