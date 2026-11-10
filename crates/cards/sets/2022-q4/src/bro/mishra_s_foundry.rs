//! Mishra's Foundry — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}: This land becomes a 2/2 Assembly-Worker artifact creature until end of turn. It's still a land.
//! Oracle: {1}, {T}: Target attacking Assembly-Worker gets +2/+2 until end of turn.
//! Set: BRO #265 — The Brothers' War | Scryfall ID: da7699b2-e1af-4bc0-8c5b-84ba3e868d7c | Oracle ID: b43e9772-6ad4-49c7-9557-b18ee1e4587d
// IMPLEMENTED — {C} mana; the {2} animation sets P/T to 2/2 and adds artifact,
// creature and Assembly-Worker until end of turn; {1}, {T} pumps an attacking
// Assembly-Worker by +2/+2.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::MISHRA_S_FOUNDRY,
    oracle_id = "b43e9772-6ad4-49c7-9557-b18ee1e4587d",
    scryfall_id = "da7699b2-e1af-4bc0-8c5b-84ba3e868d7c",
    faces = &[face!(name = "Mishra's Foundry", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}"),
            &[
                Effect::SetPTFilter {
                    filter: &Filter::This,
                    power: Amount::Fixed(2),
                    toughness: Amount::Fixed(2),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::ARTIFACT.union(TypeSet::CREATURE)),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::ASSEMBLY_WORKER),
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(2),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&f!(
                attacking Filter::HasSubtype(creature::ASSEMBLY_WORKER)
            )))
        ),
    ],
);
