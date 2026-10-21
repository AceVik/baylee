//! Casting of Bones — {2}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When enchanted creature dies, draw three cards, then discard one of them.
//! Set: ALL #44a — Alliances | Scryfall ID: e823c295-b66e-41c3-bd77-1a13f95e69c3 | Oracle ID: 5a747256-4215-4334-98ab-0c2e4ed92e47
// NOT SUPPORTED: restricting the discard to one of the three cards drawn this way is not supported; discards any card from your hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CASTING_OF_BONES,
    oracle_id = "5a747256-4215-4334-98ab-0c2e4ed92e47",
    scryfall_id = "e823c295-b66e-41c3-bd77-1a13f95e69c3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "attaches to target creature and when enchanted creature dies, draws three cards and discards a card from your hand; restricting the discard to one of the three cards drawn this way is not supported",
    ),
    faces = &[face!(
        name = "Casting of Bones",
        mana_cost = mana!("{2}{B}"),
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
            Trigger::Dies(&Filter::AttachedToBySource),
            &[
                Effect::draw(3),
                Effect::DiscardForPlayers {
                    who: PlayerRel::You,
                    count: 1,
                },
            ],
        ),
    ],
);
