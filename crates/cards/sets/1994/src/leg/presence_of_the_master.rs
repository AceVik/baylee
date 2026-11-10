//! Presence of the Master — {3}{W} — Enchantment
//! Oracle: Whenever a player casts an enchantment spell, counter it.
//! Set: USG #32 — Urza's Saga | Scryfall ID: 849adb29-61ad-4307-98b9-61e33aec6500 | Oracle ID: e1aad679-93ec-420e-881b-35ebc99763a2
// IMPLEMENTED — the cast trigger counters the enchantment spell.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRESENCE_OF_THE_MASTER,
    oracle_id = "e1aad679-93ec-420e-881b-35ebc99763a2",
    scryfall_id = "849adb29-61ad-4307-98b9-61e33aec6500",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Presence of the Master",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::ENCHANTMENT),
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::EventObject))
    )],
);
