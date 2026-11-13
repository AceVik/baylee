//! Fixtures combining a library permission with additional life payments.
use super::*;

pub(super) fn definitions() -> [&'static CardDef; 3] {
    [
        card(
            LIFE_KICKER,
            FaceDef {
                mandatory_additional_costs: FIXED_LIFE_COST,
                additional_costs: LIFE_KICKER_COST,
                ..face("Life kicker", "{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            NOTHING,
        ),
        card(
            LIFE_X_BUYBACK,
            FaceDef {
                mandatory_additional_costs: LIFE_X_COST,
                buyback: Some(Cost {
                    mana: ManaCost::ZERO,
                    parts: FIXED_LIFE_COST,
                }),
                ..face("Life X and life buyback", "{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            NOTHING,
        ),
        Box::leak(Box::new(CardDef {
            index: CardIndex::new(BACK_LIFE_KICKER),
            oracle_id: "test",
            scryfall_id: "test",
            faces: Box::leak(Box::new([
                land_face("Life kicker front"),
                FaceDef {
                    abilities: NOTHING,
                    additional_costs: BACK_LIFE_KICKER_COST,
                    ..face("Life kicker back", "{2}{U}", TypeSet::SORCERY)
                },
            ])),
            abilities: NOTHING,
            ..CardDef::DEFAULT
        })),
    ]
}
