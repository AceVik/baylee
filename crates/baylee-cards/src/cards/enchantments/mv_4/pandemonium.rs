//! Pandemonium — {3}{R} — Enchantment
//! Oracle: Whenever a creature enters, that creature's controller may have it deal damage equal to its power to any target of their choice.
//! Set: TPR #149 — Tempest Remastered | Scryfall ID: 45ad514e-28a6-4816-b764-69ef89e117f1 | Oracle ID: 073551ca-2e84-4d49-8484-9890718cc483
// PARTIAL: Triggers when a creature you control enters to let it deal damage equal to its power to any target; letting an opponent choose targets when their creature enters is not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PANDEMONIUM,
    oracle_id = "073551ca-2e84-4d49-8484-9890718cc483",
    scryfall_id = "45ad514e-28a6-4816-b764-69ef89e117f1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Triggers when a creature you control enters to let it deal damage equal to its power to any target; letting an opponent choose targets when their creature enters is not supported.",
    ),
    faces = &[face!(
        name = "Pandemonium",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&Filter::YOUR_CREATURE),
        &[Effect::MayDo {
            effects: &[Effect::EventObjectDealsDamageEqualToPower {
                target: TargetSpec::AnyTarget,
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    ),],
);
