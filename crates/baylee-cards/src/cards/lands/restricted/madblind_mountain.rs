//! Madblind Mountain — (no cost) — Land — Mountain
//! Oracle: ({T}: Add {R}.)
//! Oracle: This land enters tapped.
//! Oracle: {R}, {T}: Shuffle your library. Activate only if you control two or more red permanents.
//! Set: SHM #274 — Shadowmoor | Scryfall ID: 513adae2-6436-4284-9f23-87ef627e81b7 | Oracle ID: 0ee0b090-3f1e-49d6-bcad-91e0cf1d12ae
// PARTIAL — enters tapped, and taps for {R} off its Mountain land type
// (CR 305.6). The {R}, {T} ability is dropped: no Effect shuffles a library.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MADBLIND_MOUNTAIN,
    oracle_id = "0ee0b090-3f1e-49d6-bcad-91e0cf1d12ae",
    scryfall_id = "513adae2-6436-4284-9f23-87ef627e81b7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no Effect variant shuffles a library, so the {R}, {T} ability is not implemented"
    ),
    faces = &[face!(
        name = "Madblind Mountain",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::MOUNTAIN],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    // NOT SUPPORTED: "{R}, {T}: Shuffle your library. Activate only if you
    // control two or more red permanents." — the condition itself would be a
    // Condition::ControlCount, and the cost a cost!("{R}", TapSelf), but
    // nothing in the vocabulary shuffles a library on its own
    // (SearchLibrary shuffles only as part of a search), so the ability comes
    // off the card rather than being offered as one that spends {R} and does
    // nothing.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
);
