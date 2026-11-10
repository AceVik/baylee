//! Errant Minion — {2}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, that player may pay any amount of mana. This Aura deals 2 damage to that player. Prevent X of that damage, where X is the amount of mana that player paid this way.
//! Set: ICE #68 — Ice Age | Scryfall ID: 61648ddb-6efb-43d0-b2b1-418cc957854c | Oracle ID: 80459049-bd89-4736-82f8-03c603644033
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ERRANT_MINION,
    oracle_id = "80459049-bd89-4736-82f8-03c603644033",
    scryfall_id = "61648ddb-6efb-43d0-b2b1-418cc957854c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Errant Minion",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached,
            },
            &[Effect::PayManaToPreventDamage {
                player: PlayerRel::ActivePlayer,
                amount: Amount::Fixed(2),
            }],
        ),
    ],
);
