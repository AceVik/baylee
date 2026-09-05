//! Blinkmoth Nexus — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}: This land becomes a 1/1 Blinkmoth artifact creature with flying until end of turn. It's still a land.
//! Oracle: {1}, {T}: Target Blinkmoth creature gets +1/+1 until end of turn.
//! Set: 2XM #311 — Double Masters | Scryfall ID: 3ac535c1-9ef3-45b5-8959-7e79589d47ad | Oracle ID: 40d45c02-6416-4e19-8fe3-0ddadf5ba627
// IMPLEMENTED — {C} mana ability; {1} animate into 1/1 flying Blinkmoth artifact creature; {1},{T} pump target Blinkmoth.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TARGET_BLINKMOTH: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::BLINKMOTH),
]);

card! {
    index: 286,
    oracle_id: "40d45c02-6416-4e19-8fe3-0ddadf5ba627",
    scryfall_id: "3ac535c1-9ef3-45b5-8959-7e79589d47ad",
    faces: &[
    face! {
        name: "Blinkmoth Nexus",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[],
            },
            &[
                Effect::CreateContinuousEffect {
                    layer: Layer::Type,
                    filter: &Filter::This,
                    modifier: Modifier::AddType(TypeSet::CREATURE),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::CreateContinuousEffect {
                    layer: Layer::Type,
                    filter: &Filter::This,
                    modifier: Modifier::AddType(TypeSet::ARTIFACT),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::CreateContinuousEffect {
                    layer: Layer::Type,
                    filter: &Filter::This,
                    modifier: Modifier::AddSubtype(subtypes::creature::BLINKMOTH),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::CreateContinuousEffect {
                    layer: Layer::Ability,
                    filter: &Filter::This,
                    modifier: Modifier::AddKeyword(KeywordSet::FLYING),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::CreateContinuousEffect {
                    layer: Layer::PtSet,
                    filter: &Filter::This,
                    modifier: Modifier::SetPT(1, 1),
                    duration: Duration::UntilEndOfTurn,
                },
            ],
        ),
        activated!(
            Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target: Some(TargetSpec::Object(&TARGET_BLINKMOTH)),
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 286);
        assert_eq!(CARD.oracle_id, "40d45c02-6416-4e19-8fe3-0ddadf5ba627");
        assert_eq!(CARD.scryfall_id, "3ac535c1-9ef3-45b5-8959-7e79589d47ad");
        assert_eq!(CARD.faces[0].name, "Blinkmoth Nexus");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 3);
    }
}
