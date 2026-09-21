//! Mobilized District — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}: This land becomes a 3/3 Citizen creature with vigilance until end of turn. It's still a land. This ability costs {1} less to activate for each legendary creature and planeswalker you control.
//! Set: CMM #1011 — Commander Masters | Scryfall ID: c7921b97-083b-480a-a7df-f2ab82e6f8fb | Oracle ID: eb2094cf-b4be-4f52-8615-e179ef7c741d
// PARTIAL — {T}: Add {C}, and the manland animation written as four
// continuous effects on the source (type, subtype, vigilance, 3/3); the
// printed activation-cost reduction has no vocabulary.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::MOBILIZED_DISTRICT,
    oracle_id = "eb2094cf-b4be-4f52-8615-e179ef7c741d",
    scryfall_id = "c7921b97-083b-480a-a7df-f2ab82e6f8fb",
    faces = &[face!(name = "Mobilized District", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {1}-less activation cost for each legendary creature and planeswalker you control: no cost reduction a card can state"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "This ability costs {1} less to activate for each
        // legendary creature and planeswalker you control." CostReduction
        // names only NotStartingPlayer, and no field on a FaceDef reads it.
        activated!(
            cost!("{4}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::CITIZEN),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::VIGILANCE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(3, 3),
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
    ],
);
