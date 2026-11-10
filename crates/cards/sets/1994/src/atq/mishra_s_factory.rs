//! Mishra's Factory — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}: This land becomes a 2/2 Assembly-Worker artifact creature until end of turn. It's still a land.
//! Oracle: {T}: Target Assembly-Worker creature gets +1/+1 until end of turn.
//! Set: DMR #251 — Dominaria Remastered | Scryfall ID: 6e9fec20-a52c-42c0-9928-c572d9e1b21f | Oracle ID: 5963e0ef-e0bc-4611-ad4f-813a4c0eacfb
// IMPLEMENTED — colorless mana; the {1} self-animation as three continuous
// effects on the source (artifact and creature types added in layer 4, the
// Assembly-Worker subtype in layer 4, base P/T set to 2/2 in layer 7b, land
// type kept because types are only added); and a targeted +1/+1 to an
// Assembly-Worker creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISHRA_S_FACTORY,
    oracle_id = "5963e0ef-e0bc-4611-ad4f-813a4c0eacfb",
    scryfall_id = "6e9fec20-a52c-42c0-9928-c572d9e1b21f",
    faces = &[face!(name = "Mishra's Factory", types = TypeSet::LAND,),],
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
                    Modifier::AddSubtype(subtypes::creature::ASSEMBLY_WORKER),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(2, 2),
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
        activated!(
            Cost::TAP,
            &[Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::CREATURE,
                Filter::HasSubtype(subtypes::creature::ASSEMBLY_WORKER),
            ])))
        ),
    ],
);
