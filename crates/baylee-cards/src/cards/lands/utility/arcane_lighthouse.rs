//! Arcane Lighthouse — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Until end of turn, creatures your opponents control lose hexproof and shroud and can't have hexproof or shroud.
//! Set: SOC #361 — Secrets of Strixhaven Commander | Scryfall ID: e2384e42-2442-4b40-9bae-8db470d2fb8c | Oracle ID: 30ac68e6-160a-41f9-9f0f-0e0eef383150
// PARTIAL — {T}: Add {C}, and {1}, {T} strips hexproof and shroud from each
// opponent's creature until end of turn; the printed "can't have hexproof or
// shroud" half has no variant.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARCANE_LIGHTHOUSE,
    oracle_id = "30ac68e6-160a-41f9-9f0f-0e0eef383150",
    scryfall_id = "e2384e42-2442-4b40-9bae-8db470d2fb8c",
    faces = &[face!(name = "Arcane Lighthouse", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no Modifier says an object can't have hexproof or shroud for the rest of the turn"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "and can't have hexproof or shroud" — RemoveKeyword
        // takes off what is already on the board, and nothing here stops a
        // keyword granted later in the same turn.
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::continuous(
                &Filter::OPPONENT_CREATURE,
                Modifier::RemoveKeyword(KeywordSet::HEXPROOF.union(KeywordSet::SHROUD)),
                Duration::UntilEndOfTurn,
            )]
        ),
    ],
);
