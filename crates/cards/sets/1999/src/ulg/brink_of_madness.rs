//! Brink of Madness — {2}{B}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, if you have no cards in hand, sacrifice this enchantment and target opponent discards their hand.
//! Set: ULG #50 — Urza's Legacy | Scryfall ID: ff5391d8-b546-4159-955e-16bb58052311 | Oracle ID: 3209ad08-5482-4dd4-a02e-1dbe99ce6b54
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRINK_OF_MADNESS,
    oracle_id = "3209ad08-5482-4dd4-a02e-1dbe99ce6b54",
    scryfall_id = "ff5391d8-b546-4159-955e-16bb58052311",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Brink of Madness",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[
            Effect::SacrificeSelf,
            Effect::DiscardHand {
                who: PlayerRel::TargetPlayerOrController,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
        condition = Some(Condition::HandSizeAtMost(0)),
    )],
);
