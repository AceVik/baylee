//! Pendelhaven — (no cost) — Legendary Land
//! Oracle: {T}: Add {G}.
//! Oracle: {T}: Target 1/1 creature gets +1/+2 until end of turn.
//! Set: A25 #244 — Masters 25 | Scryfall ID: acf85879-4d14-4d86-978c-b155c47b7dcd | Oracle ID: f70e72e1-9abe-485b-9fea-e8b35352f5b3
// PARTIAL — the mana ability is built; the second ability cannot be said,
// because no Filter predicate reads a creature's *power*.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PENDELHAVEN,
    oracle_id = "f70e72e1-9abe-485b-9fea-e8b35352f5b3",
    scryfall_id = "acf85879-4d14-4d86-978c-b155c47b7dcd",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Pendelhaven",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "{T}: Target 1/1 creature gets +1/+2 until end of turn — \"1/1\" is a power *and* a \
         toughness and Filter has no power predicate"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),],
);

// NOT SUPPORTED: "{T}: Target 1/1 creature gets +1/+2 until end of turn."
// The nearest variant is `Filter::ToughnessAtMost(1)`, and it is a different
// card: it admits a 2/1 and a 0/1, where the printing admits neither. The
// filter vocabulary has `ToughnessAtMost` and nothing about power —
// `Amount::TargetPower` reads a power, `Filter` never asks for one — so the
// ability comes off rather than being approximated, and the card stays
// unplayable rather than playable-and-wrong.
