//! Branch of Vitu-Ghazi — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: Disguise {3} (You may cast this card face down for {3} as a 2/2 creature with ward {2}. Turn it face up any time for its disguise cost.)
//! Oracle: When this land is turned face up, add two mana of any one color. Until end of turn, you don't lose this mana as steps and phases end.
//! Set: MKM #258 — Murders at Karlov Manor | Scryfall ID: 73a8169f-b858-47a5-9c76-2e7c50ad4ecd | Oracle ID: 7a30316b-dcd5-4a4b-b959-eecde7ca92e7
// IMPLEMENTED — disguise, ward {2}, face-up special action and mana retention.
// Engine tests: ward_tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRANCH_OF_VITU_GHAZI,
    oracle_id = "7a30316b-dcd5-4a4b-b959-eecde7ca92e7",
    scryfall_id = "73a8169f-b858-47a5-9c76-2e7c50ad4ecd",
    faces = &[face!(
        name = "Branch of Vitu-Ghazi",
        types = TypeSet::LAND,
        disguise = Some(mana!("{3}"))
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        triggered!(
            Trigger::TurnedFaceUp,
            &[Effect::AddMana {
                source: baylee_cards_dsl::ManaSource::Choice(baylee_cards_dsl::ALL_MANA_COLORS),
                amount: Amount::Fixed(2),
                combination: false,
                restriction: Some(baylee_cards_dsl::ManaRestriction {
                    filter: &Filter::Any,
                    rider: SpendRider::None,
                    restricts: false,
                    until_end_of_turn: true,
                }),
            }]
        ),
    ],
);
