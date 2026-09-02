//! Recurring Nightmare — {2}{B} — Enchantment
//! Oracle: Sacrifice a creature, Return this enchantment to its owner's hand: Return target creature card from your graveyard to the battlefield. Activate only as a sorcery.
//! Set: TPR #113 — Tempest Remastered | Scryfall ID: b50e1800-a45c-43bd-8886-8a06145d9346 | Oracle ID: a6708b11-1bcd-4208-a967-fe91f2e3313c
// IMPLEMENTED — sacrifice + bounce-to-hand cost, sorcery-speed
// reanimation.

static CREATURE_YOU_CONTROL: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 127,
    oracle_id: "a6708b11-1bcd-4208-a967-fe91f2e3313c",
    scryfall_id: "b50e1800-a45c-43bd-8886-8a06145d9346",
    faces: &[face! {
        name: "Recurring Nightmare",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::ENCHANTMENT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[activated!(Cost {
            mana: ManaCost::ZERO,
            parts: &[
                CostPart::Sacrifice(&CREATURE_YOU_CONTROL),
                CostPart::ReturnSelfToHand,
            ],
        }, &[Effect::GraveyardToBattlefield {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
        }], target: Some(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You)), timing: ActivationTiming::SorcerySpeed)],
}
