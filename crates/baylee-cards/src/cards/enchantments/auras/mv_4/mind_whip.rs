//! Mind Whip — {2}{B}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, that player may pay {3}. If they don't, this Aura deals 2 damage to that player and you tap that creature.
//! Set: ICE #149 — Ice Age | Scryfall ID: 3f3ff5fb-4126-4a18-b540-2beaae382e59 | Oracle ID: e26f7a58-18ae-432b-bc3f-c410512eb6bf
// IMPLEMENTED — Aura upkeep trigger offering enchanted creature's controller {3} to avoid 2 damage and tapping enchanted creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MIND_WHIP,
    oracle_id = "e26f7a58-18ae-432b-bc3f-c410512eb6bf",
    scryfall_id = "3f3ff5fb-4126-4a18-b540-2beaae382e59",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mind Whip",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached,
            },
            &[Effect::PlayerMayPayThenElse {
                player: PlayerRel::ControllerOfAttached,
                mana: Amount::Fixed(3),
                paid: &[],
                unpaid: &[
                    Effect::DealDamage {
                        amount: Amount::Fixed(2),
                        target: TargetSpec::Player(PlayerRel::ControllerOfAttached),
                    },
                    Effect::TapAll {
                        filter: &Filter::AttachedToBySource,
                    },
                ],
            }],
        ),
    ],
);
