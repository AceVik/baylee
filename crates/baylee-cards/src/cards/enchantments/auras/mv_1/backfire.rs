//! Backfire — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature deals damage to you, this Aura deals that much damage to that creature's controller.
//! Set: 4ED #62 — Fourth Edition | Scryfall ID: ad34094d-a7ec-4b04-a288-4d4f1a07fc6b | Oracle ID: e04fa58e-be7a-4c9c-98d0-48241742a41b
// PARTIAL — enchant creature is written; the damage trigger is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BACKFIRE,
    oracle_id = "e04fa58e-be7a-4c9c-98d0-48241742a41b",
    scryfall_id = "ad34094d-a7ec-4b04-a288-4d4f1a07fc6b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "no Trigger hears a filtered source deal damage: \
         Trigger::PlayerDealtDamage(You) fires for every source that damages \
         you and no filter narrows it to the enchanted creature, so the \
         trigger is off the card"
    ),
    faces = &[face!(
        name = "Backfire",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "Whenever enchanted creature deals damage to you, this
    // Aura deals that much damage to that creature's controller." — the
    // effect is sayable (`Effect::DealDamage` at
    // `PlayerRel::ControllerOfEvent` with `Amount::EventAmount`), but no
    // `Trigger` names a damage source: `PlayerDealtDamage` ignores which
    // source dealt the damage, and `DealsDamageToOpponent`/
    // `DealsCombatDamageToOpponent` hear only an opponent being dealt to.
    abilities = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE)
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
