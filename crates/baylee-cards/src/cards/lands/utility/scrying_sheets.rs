//! Scrying Sheets — (no cost) — Snow Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{S}, {T}: Look at the top card of your library. If that card is snow, you may reveal it and put it into your hand. ({S} can be paid with one mana from a snow source.)
//! Set: CSP #149 — Coldsnap | Scryfall ID: 19e95422-c6fe-4750-916b-43bf22ae193a | Oracle ID: 0f98e055-ab61-4314-aae8-9d3c19f66acf
// PARTIAL — the {T}: Add {C} mana ability is built; the second ability has no
// DSL shape and is left off (see the NOT SUPPORTED line).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCRYING_SHEETS,
    oracle_id = "0f98e055-ab61-4314-aae8-9d3c19f66acf",
    scryfall_id = "19e95422-c6fe-4750-916b-43bf22ae193a",
    faces = &[face!(
        name = "Scrying Sheets",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::SNOW,
    ),],
    coverage = Coverage::Partial(
        "the {1}{S}, {T} ability is not expressible — see the NOT SUPPORTED line"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{1}{S}, {T}: Look at the top card of your library. If
        // that card is snow, you may reveal it and put it into your hand." — no
        // Effect reads the top card of a library and branches on its printed
        // characteristics: Effect::LookAtTopPick is unconditional, reveals
        // nothing, and sends everything it does not keep to the bottom.
    ],
);
