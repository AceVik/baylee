//! Sigil of Sleep — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature deals damage to a player, return target creature that player controls to its owner's hand.
//! Set: DDI #25 — Duel Decks: Venser vs. Koth | Scryfall ID: 13063520-6e2f-4eaa-b84f-a38e5bf486f8 | Oracle ID: 51b47053-d169-4627-982f-f29b8630d75f

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SIGIL_OF_SLEEP,
    oracle_id = "51b47053-d169-4627-982f-f29b8630d75f",
    scryfall_id = "13063520-6e2f-4eaa-b84f-a38e5bf486f8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sigil of Sleep",
        mana_cost = mana!("{U}"),
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
            Trigger::DealsDamageToPlayer {
                source: &Filter::AttachedToBySource,
                player: PlayerRel::EachPlayer,
            },
            &[Effect::ReturnToHand {
                target: TargetSpec::ObjectOfEventPlayer(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::ObjectOfEventPlayer(
                &Filter::CREATURE
            ))),
        ),
    ],
);
