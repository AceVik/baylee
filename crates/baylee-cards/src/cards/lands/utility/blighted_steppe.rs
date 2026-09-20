//! Blighted Steppe — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}{W}, {T}, Sacrifice this land: You gain 2 life for each creature you control.
//! Set: BFZ #232 — Battle for Zendikar | Scryfall ID: f43df8ee-d3e4-419e-aed0-1059d95f9cab | Oracle ID: db16a2fb-dc42-4086-9928-52076043097f
// PARTIAL — the mana ability is the whole first line; the sacrifice clause
// wants 2 life for each creature, and `Amount` counts objects without
// anything that multiplies the count.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLIGHTED_STEPPE,
    oracle_id = "db16a2fb-dc42-4086-9928-52076043097f",
    scryfall_id = "f43df8ee-d3e4-419e-aed0-1059d95f9cab",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(name = "Blighted Steppe", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "the sacrifice ability: 2 life for each creature you control — Amount::CountOf counts \
         creatures but no amount scales a count, so the printed 2 per creature has no spelling"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: {3}{W}, {T}, Sacrifice this land: You gain 2 life for each
// creature you control.
