//! El-Hajjâj — {1}{B}{B} — Creature — Human Wizard
//! Oracle: Whenever this creature deals damage, you gain that much life.
//! Set: 4ED #134 — Fourth Edition | Scryfall ID: cf371bd2-89ad-487e-8f27-37a6e75ca0f5 | Oracle ID: f92c9a5d-853f-4157-89ba-8d8c8033c533
// PARTIAL — the damage trigger is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EL_HAJJAJ,
    oracle_id = "f92c9a5d-853f-4157-89ba-8d8c8033c533",
    scryfall_id = "cf371bd2-89ad-487e-8f27-37a6e75ca0f5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "El-Hajjâj",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "no Trigger hears this source deal damage to any target, so the \
         life-gain trigger has no event it can listen to"
    ),
    // NOT SUPPORTED: "Whenever this creature deals damage, you gain that
    // much life." — the gain itself is sayable (`Effect::GainLife` with
    // `Amount::EventAmount`), but the event is not:
    // `Trigger::DealsDamageToOpponent` and
    // `Trigger::DealsCombatDamageToOpponent` hear only an opponent being
    // dealt to, so damage to creatures, battles or to the source's own
    // controller would be dropped. No `Trigger` names this source dealing
    // damage to anything, so the trigger comes off the card.
);
