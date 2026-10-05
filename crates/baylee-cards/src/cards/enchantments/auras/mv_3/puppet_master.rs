//! Puppet Master — {U}{U}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When enchanted creature dies, return that card to its owner's hand. If that card is returned to its owner's hand this way, you may pay {U}{U}{U}. If you do, return this card to its owner's hand.
//! Set: CHR #23 — Chronicles | Scryfall ID: b401d253-4e51-42db-91cc-ac8cc3d06ae6 | Oracle ID: 42c7e933-f169-444b-917d-b4dca918d989
// NOT SUPPORTED: paying {U}{U}{U} does not return Puppet Master to its owner's hand.
// Otherwise implemented — enchant creature; when it dies the card is returned from the
// graveyard to its owner's hand, and only when it arrived that way the Aura's
// controller may pay {U}{U}{U} to return Puppet Master the same way.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PUPPET_MASTER,
    oracle_id = "42c7e933-f169-444b-917d-b4dca918d989",
    scryfall_id = "b401d253-4e51-42db-91cc-ac8cc3d06ae6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage =
        Coverage::Partial("paying {U}{U}{U} does not return Puppet Master to its owner's hand"),
    faces = &[face!(
        name = "Puppet Master",
        mana_cost = mana!("{U}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::Dies(&Filter::AttachedToBySource),
            &[
                Effect::GraveyardToHand {
                    target: TargetSpec::EventObject
                },
                // "If that card is returned to its owner's hand this way":
                // the event object is the same card, so a card in hand is
                // one the effect above actually moved there.
                Effect::IfEventObjectMatches {
                    filter: &Filter::InZone(ZoneRef::Hand),
                    then: &[Effect::PlayerMayPayManaThen {
                        player: PlayerRel::You,
                        cost: mana!("{U}{U}{U}"),
                        effects: &[Effect::ReturnToHand {
                            target: TargetSpec::ThisObject
                        }],
                    }],
                },
            ]
        ),
    ],
);
