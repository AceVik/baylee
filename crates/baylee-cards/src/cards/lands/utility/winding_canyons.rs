//! Winding Canyons — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: You may cast creature spells this turn as though they had flash.
//! Set: WTH #167 — Weatherlight | Scryfall ID: f26672a8-f4ff-4c64-bb3e-f5072bbc9e3e | Oracle ID: 622e2561-48b1-4aca-9abb-9a3c284dcceb

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINDING_CANYONS,
    oracle_id = "622e2561-48b1-4aca-9abb-9a3c284dcceb",
    scryfall_id = "f26672a8-f4ff-4c64-bb3e-f5072bbc9e3e",
    faces = &[face!(name = "Winding Canyons", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::SpellsHaveFlash(Some(TypeSet::CREATURE)),
                Duration::UntilEndOfTurn,
            )]
        ),
    ],
);
