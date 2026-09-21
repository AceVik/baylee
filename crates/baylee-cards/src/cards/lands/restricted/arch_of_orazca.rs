//! Arch of Orazca — (no cost) — Land
//! Oracle: Ascend (If you control ten or more permanents, you get the city's blessing for the rest of the game.)
//! Oracle: {T}: Add {C}.
//! Oracle: {5}, {T}: Draw a card. Activate only if you have the city's blessing.
//! Set: LCC #319 — The Lost Caverns of Ixalan Commander | Scryfall ID: 581dcadd-7de4-4b39-bab0-d3567194a252 | Oracle ID: 3bb518ff-399b-4ce7-b9ad-a1d563dd7792
// IMPLEMENTED — {T}: Add {C}. Ascend and the city's-blessing-gated draw are
// NOT SUPPORTED (no DSL condition reads the blessing).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARCH_OF_ORAZCA,
    oracle_id = "3bb518ff-399b-4ce7-b9ad-a1d563dd7792",
    scryfall_id = "581dcadd-7de4-4b39-bab0-d3567194a252",
    faces = &[face!(name = "Arch of Orazca", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "Ascend / the city's blessing is not expressible: no effect grants it and no Condition variant reads it"
    ),
    abilities = &[
        // NOT SUPPORTED: Ascend — nothing in the DSL grants or tracks the
        // city's blessing for the rest of the game.
        // NOT SUPPORTED: {5}, {T}: Draw a card. Activate only if you have
        // the city's blessing — the activation condition is unreadable, so
        // the ability cannot be offered (an unconditional version would be a
        // rules bug, not an approximation).
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
