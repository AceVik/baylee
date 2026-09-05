//! Blighted Fen — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}{B}, {T}, Sacrifice this land: Target opponent sacrifices a creature of their choice.
//! Set: BFZ #230 — Battle for Zendikar | Scryfall ID: 29d02950-cd50-4662-97af-3106598dc3c4 | Oracle ID: b8f3da11-7c8f-4846-98a6-204bfd8d572b
// IMPLEMENTED — tap for {C}, or pay {4}{B} and sacrifice to make target opponent sacrifice a creature.

use baylee_cards_dsl::prelude::*;

card! {
    index: 281,
    oracle_id: "b8f3da11-7c8f-4846-98a6-204bfd8d572b",
    scryfall_id: "29d02950-cd50-4662-97af-3106598dc3c4",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Blighted Fen",
        types: TypeSet::LAND,
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{4}{B}"),
                parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
            },
            &[Effect::SacrificeFilter {
                who: PlayerRel::Chosen,
                filter: &Filter::CREATURE,
            }],
            target: Some(TargetSpec::AnyOpponent),
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 281);
        assert_eq!(CARD.oracle_id, "b8f3da11-7c8f-4846-98a6-204bfd8d572b");
        assert_eq!(CARD.scryfall_id, "29d02950-cd50-4662-97af-3106598dc3c4");
        assert_eq!(CARD.faces[0].name, "Blighted Fen");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.color_identity, ColorSet::from_slice(&[Color::Black]));
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}

