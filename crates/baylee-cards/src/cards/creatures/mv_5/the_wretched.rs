//! The Wretched — {3}{B}{B} — Creature — Demon
//! Oracle: At end of combat, gain control of all creatures blocking this creature for as long as you control this creature.
//! Set: ME3 #78 — Masters Edition III | Scryfall ID: edf30a58-f77c-4887-a0b2-9d939243d86e | Oracle ID: 27af77e5-6680-43d3-9202-40be9b5d14a0
// PARTIAL — the ability is off the card (see NOT SUPPORTED below).
// NOT SUPPORTED: "At end of combat, gain control of all creatures blocking
// this creature for as long as you control this creature." — no filter
// names the creatures blocking the source. `Filter::Blocking` matches every
// blocker on the battlefield, and the combat pairs that say which attacker
// each one blocks are not reachable from a `Filter`, so "all creatures
// blocking this creature" cannot be selected: sweeping every blocker would
// steal the blockers of the other attackers as well. The two surrounding
// pieces exist — `Trigger::StepBegin { step: StepKind::CombatEnd, .. }` for
// "at end of combat" and `Modifier::GainControl` (layer 2) under
// `Duration::WhileYouControlSource` — but the affected set does not.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_WRETCHED,
    oracle_id = "27af77e5-6680-43d3-9202-40be9b5d14a0",
    scryfall_id = "edf30a58-f77c-4887-a0b2-9d939243d86e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "no filter names a creature blocking the source, so the affected set \
         of the end-of-combat control change cannot be stated; the trigger \
         and the WhileYouControlSource duration both exist"
    ),
    faces = &[face!(
        name = "The Wretched",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DEMON],
        power = Some(2),
        toughness = Some(5),
    ),],
);
