//! Dragon-Cursed Halls — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Until end of turn, target creature gains "Whenever this creature deals combat damage to a player, create a Treasure token."
//! Set: HOC #8 — The Hobbit Eternal | Scryfall ID: 506b9df7-8236-4c6e-aebc-6b7e6fcd7e88 | Oracle ID: 5be7a4d5-33b7-464b-8851-d4ad35302e62
// IMPLEMENTED — {C} mana + grant combat-damage Treasure trigger until EOT.

use baylee_cards_dsl::prelude::*;

use crate::tokens::TREASURE;

static TREASURE_TRIGGER_EFFECTS: &[Effect] = &[Effect::CreateToken {
    token: &TREASURE,
}];

card! {
    index: 435,
    oracle_id: "5be7a4d5-33b7-464b-8851-d4ad35302e62",
    scryfall_id: "506b9df7-8236-4c6e-aebc-6b7e6fcd7e88",
    faces: &[
    face! {
        name: "Dragon-Cursed Halls",
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
                filter: &Filter::This,
                modifier: Modifier::GrantTriggered {
                    trigger: Trigger::DealsCombatDamageToPlayer(&Filter::This),
                    effects: TREASURE_TRIGGER_EFFECTS,
                    target: None,
                },
                duration: Duration::UntilEndOfTurn,
            }],
            target: Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 435);
        assert_eq!(CARD.oracle_id, "5be7a4d5-33b7-464b-8851-d4ad35302e62");
        assert_eq!(CARD.scryfall_id, "506b9df7-8236-4c6e-aebc-6b7e6fcd7e88");
        assert_eq!(CARD.faces[0].name, "Dragon-Cursed Halls");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
