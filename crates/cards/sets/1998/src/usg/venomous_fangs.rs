//! Venomous Fangs — {2}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature deals damage to a creature, destroy the other creature.
//! Set: USG #280 — Urza's Saga | Scryfall ID: f5ae1348-c063-4ec8-9d8c-3d19a6421800 | Oracle ID: a1ee715e-bab5-4edc-89ba-610cf71bb048
// PARTIAL: triggering on noncombat damage dealt by enchanted creature to a creature is not in Trigger; triggers when enchanted creature deals combat damage to a creature and destroys that creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VENOMOUS_FANGS,
    oracle_id = "a1ee715e-bab5-4edc-89ba-610cf71bb048",
    scryfall_id = "f5ae1348-c063-4ec8-9d8c-3d19a6421800",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "triggering on noncombat damage dealt by enchanted creature to a creature is not in Trigger; triggers when enchanted creature deals combat damage to a creature and destroys that creature",
    ),
    faces = &[face!(
        name = "Venomous Fangs",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: Noncombat damage by the enchanted creature is not available as a creature-recipient trigger.
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        triggered!(
            Trigger::DealsCombatDamageToCreature(&Filter::AttachedToBySource),
            &[Effect::destroy(TargetSpec::EventObject)],
        ),
    ],
);
