//! Arcane Lighthouse — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Until end of turn, creatures your opponents control lose hexproof and shroud and can't have hexproof or shroud.
//! Set: SOC #361 — Secrets of Strixhaven Commander | Scryfall ID: e2384e42-2442-4b40-9bae-8db470d2fb8c | Oracle ID: 30ac68e6-160a-41f9-9f0f-0e0eef383150
// IMPLEMENTED — {C} mana ability + removes hexproof and shroud from opponents' creatures until EOT.

use baylee_cards_dsl::prelude::*;

card! {
    index: 227,
    oracle_id: "30ac68e6-160a-41f9-9f0f-0e0eef383150",
    scryfall_id: "e2384e42-2442-4b40-9bae-8db470d2fb8c",
    faces: &[
    face! {
        name: "Arcane Lighthouse",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::CreateContinuousEffect {
                layer: Layer::Ability,
                filter: &Filter::OPPONENT_CREATURE,
                modifier: Modifier::RemoveKeyword(
                    KeywordSet::HEXPROOF.union(KeywordSet::SHROUD),
                ),
                duration: Duration::UntilEndOfTurn,
            }]
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 227);
        assert_eq!(CARD.oracle_id, "30ac68e6-160a-41f9-9f0f-0e0eef383150");
        assert_eq!(CARD.scryfall_id, "e2384e42-2442-4b40-9bae-8db470d2fb8c");
        assert_eq!(CARD.faces[0].name, "Arcane Lighthouse");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
