//! Mijae Djinn — {R}{R}{R} — Creature — Djinn
//! Oracle: Whenever this creature attacks, flip a coin. If you lose the flip, remove this creature from combat and tap it.
//! Set: ME4 #127 — Masters Edition IV | Scryfall ID: b4eb4f9e-4c11-4563-9f9a-87ec93274695 | Oracle ID: 61bebdfa-5df0-4952-abf9-dc5e0e4f57ea
// PARTIAL — the attack trigger is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MIJAE_DJINN,
    oracle_id = "61bebdfa-5df0-4952-abf9-dc5e0e4f57ea",
    scryfall_id = "b4eb4f9e-4c11-4563-9f9a-87ec93274695",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mijae Djinn",
        mana_cost = mana!("{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(6),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "no DSL effect flips a coin, so there is nothing to branch on, and no \
         effect removes an object from combat"
    ),
    // NOT SUPPORTED: "Whenever this creature attacks, flip a coin. If you
    // lose the flip, remove this creature from combat and tap it." — the
    // engine's generator can flip a coin, but no Effect exposes a flip and
    // no Result/If branch can read one; no effect takes an object out of
    // combat (Effect::TapSelf would only tap it unconditionally), and
    // Trigger::Attacks would fire on every attack rather than on a lost flip.
);
