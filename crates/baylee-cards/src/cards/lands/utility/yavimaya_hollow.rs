//! Yavimaya Hollow — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {G}, {T}: Regenerate target creature.
//! Set: VMA #325 — Vintage Masters | Scryfall ID: d9fdbc02-7ab7-4f77-8a89-5a9e01eb32f5 | Oracle ID: 53d6113d-acdb-4754-9641-f7991a96c7b9
// PARTIAL — {T}: Add {C} is built; the regenerate ability is dropped, since
// nothing in the DSL expresses a regeneration shield.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::YAVIMAYA_HOLLOW,
    oracle_id = "53d6113d-acdb-4754-9641-f7991a96c7b9",
    scryfall_id = "d9fdbc02-7ab7-4f77-8a89-5a9e01eb32f5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "`{G}, {T}: Regenerate target creature` — no Effect, Modifier or readable keyword expresses a regeneration shield"
    ),
    faces = &[face!(
        name = "Yavimaya Hollow",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: {G}, {T}: Regenerate target creature. — regeneration
        // is a replacement effect ("the next time this permanent would be
        // destroyed this turn, instead tap it, remove it from combat, and
        // remove all damage from it"). There is no Effect for it, no Modifier
        // for it, and `regenerate` is not among the keyword bits the engine
        // reads, so the ability comes off the card rather than shipping an
        // activation that resolves into nothing.
    ],
);
