//! Blinkmoth Nexus — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}: This land becomes a 1/1 Blinkmoth artifact creature with flying until end of turn. It's still a land.
//! Oracle: {1}, {T}: Target Blinkmoth creature gets +1/+1 until end of turn.
//! Set: 2XM #311 — Double Masters | Scryfall ID: 3ac535c1-9ef3-45b5-8959-7e79589d47ad | Oracle ID: 40d45c02-6416-4e19-8fe3-0ddadf5ba627
// IMPLEMENTED — the {C} mana ability, the animation ({1}: artifact creature
// with flying and the Blinkmoth type, still a land) and the {1}, {T} pump of a
// target Blinkmoth creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::BLINKMOTH_NEXUS,
    oracle_id = "40d45c02-6416-4e19-8fe3-0ddadf5ba627",
    scryfall_id = "3ac535c1-9ef3-45b5-8959-7e79589d47ad",
    faces = &[face!(name = "Blinkmoth Nexus", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::ARTIFACT.union(TypeSet::CREATURE)),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::BLINKMOTH),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::FLYING),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(1, 1),
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::CREATURE,
                Filter::HasSubtype(creature::BLINKMOTH),
            ])))
        ),
    ],
);
