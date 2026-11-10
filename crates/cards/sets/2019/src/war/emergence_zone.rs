//! Emergence Zone — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice this land: You may cast spells this turn as though they had flash.
//! Set: WAR #245 — War of the Spark | Scryfall ID: ab95f6e7-b806-47fe-a071-6c38b3176d94 | Oracle ID: 7536eb66-959d-4dca-9b75-895572ef733c

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EMERGENCE_ZONE,
    oracle_id = "7536eb66-959d-4dca-9b75-895572ef733c",
    scryfall_id = "ab95f6e7-b806-47fe-a071-6c38b3176d94",
    faces = &[face!(name = "Emergence Zone", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf, SacrificeSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::SpellsHaveFlash(None),
                Duration::UntilEndOfTurn,
            )]
        ),
    ],
);
