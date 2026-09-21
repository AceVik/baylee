//! Tarnation Vista — (no cost) — Land
//! Oracle: This land enters tapped. As it enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Oracle: {1}, {T}: For each color among monocolored permanents you control, add one mana of that color.
//! Set: BIG #30 — The Big Score | Scryfall ID: 962552a1-ec34-49e2-a23d-85dfb405d5e0 | Oracle ID: b0be3f25-edb1-4299-959c-9aad909730ca
// PARTIAL — the entry modifier and the chosen-colour mana ability are built;
// the {1}, {T} ability is not expressible (see the NOT SUPPORTED line below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TARNATION_VISTA,
    oracle_id = "b0be3f25-edb1-4299-959c-9aad909730ca",
    scryfall_id = "962552a1-ec34-49e2-a23d-85dfb405d5e0",
    faces = &[face!(
        name = "Tarnation Vista",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped, EnterModifier::ChooseColor],
    )],
    coverage = Coverage::Partial(
        "the {1}, {T} ability is not expressible: no ManaSource produces one \
         mana per distinct colour among a filtered set"
    ),
    // NOT SUPPORTED: {1}, {T}: For each color among monocolored permanents you
    // control, add one mana of that color. Amount::DistinctColorsAmong counts
    // the colours, but every ManaSource names its colours up front — a
    // ManaSource::Choice list is fixed, and mana_combination would hand out
    // any N colours the player likes (the same one twice included) rather than
    // one mana of each colour actually on the battlefield.
    abilities = &[mana_ability!(&[Effect::mana_chosen()])],
);
