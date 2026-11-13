//! Blinding Angel — {3}{W}{W} — Creature — Angel
//! Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.)
//! Oracle: Whenever this creature deals combat damage to a player, that player skips their next combat phase.
//! Set: 9ED #7 — Ninth Edition | Scryfall ID: e04256f8-c275-4a24-8a95-371d424f7a01 | Oracle ID: cb479ce1-59e8-4531-9a21-f7c97d334817
// IMPLEMENTED — flying and a combat-damage trigger delaying the next combat.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLINDING_ANGEL,
    oracle_id = "cb479ce1-59e8-4531-9a21-f7c97d334817",
    scryfall_id = "e04256f8-c275-4a24-8a95-371d424f7a01",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blinding Angel",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::DealsCombatDamageToPlayer(&Filter::This),
        &[Effect::SkipNextCombatPhase {
            player: PlayerRel::EventPlayer,
        }],
    )],
);
