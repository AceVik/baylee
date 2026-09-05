//! Dread Statuary — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}: This land becomes a 4/2 Golem artifact creature until end of turn. It's still a land.
//! Set: CN2 #217 — Conspiracy: Take the Crown | Scryfall ID: 6e54bace-7484-484e-987e-8d3a9a430ab9 | Oracle ID: a9789ce4-69cf-435c-b99a-78a21609830c
// IMPLEMENTED — {C} mana ability; {4} animate into 4/2 Golem artifact creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 438,
    oracle_id: "a9789ce4-69cf-435c-b99a-78a21609830c",
    scryfall_id: "6e54bace-7484-484e-987e-8d3a9a430ab9",
    faces: &[
    face! {
        name: "Dread Statuary",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{4}"),
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
                    modifier: Modifier::AddSubtype(creature::GOLEM),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::CreateContinuousEffect {
                    layer: Layer::PtSet,
                    filter: &Filter::This,
                    modifier: Modifier::SetPT(4, 2),
                    duration: Duration::UntilEndOfTurn,
                },
            ],
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 438);
        assert_eq!(CARD.oracle_id, "a9789ce4-69cf-435c-b99a-78a21609830c");
        assert_eq!(CARD.scryfall_id, "6e54bace-7484-484e-987e-8d3a9a430ab9");
        assert_eq!(CARD.faces[0].name, "Dread Statuary");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
