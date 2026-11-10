//! Dread Statuary — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}: This land becomes a 4/2 Golem artifact creature until end of turn. It's still a land.
//! Set: CN2 #217 — Conspiracy: Take the Crown | Scryfall ID: 6e54bace-7484-484e-987e-8d3a9a430ab9 | Oracle ID: a9789ce4-69cf-435c-b99a-78a21609830c
// IMPLEMENTED — {T}: Add {C}; {4} turns this land into a 4/2 Golem artifact
// creature until end of turn (layer 4 types + subtype, layer 7b P/T), and it
// stays a land because adding types takes nothing away.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::DREAD_STATUARY,
    oracle_id = "a9789ce4-69cf-435c-b99a-78a21609830c",
    scryfall_id = "6e54bace-7484-484e-987e-8d3a9a430ab9",
    faces = &[face!(name = "Dread Statuary", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE.union(TypeSet::ARTIFACT)),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::GOLEM),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(4, 2),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
    ],
);
