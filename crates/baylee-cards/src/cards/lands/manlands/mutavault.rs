//! Mutavault — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}: This land becomes a 2/2 creature with all creature types until end of turn. It's still a land.
//! Set: CLB #903 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 52cc2f10-142d-4e6a-984e-b25f566cc960 | Oracle ID: 6b3cc59a-7ea5-4eb5-9bf9-5a9c07f80e2b
// IMPLEMENTED — {C} mana ability, plus the {1} animation: adds the creature
// type, every creature type, and sets P/T to 2/2 until end of turn (the land
// type is never removed, so it is still a land).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MUTAVAULT,
    oracle_id = "6b3cc59a-7ea5-4eb5-9bf9-5a9c07f80e2b",
    scryfall_id = "52cc2f10-142d-4e6a-984e-b25f566cc960",
    faces = &[face!(name = "Mutavault", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AllCreatureTypes,
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(2, 2),
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
    ],
);
