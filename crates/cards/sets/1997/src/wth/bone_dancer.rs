//! Bone Dancer — {1}{B}{B} — Creature — Zombie
//! Oracle: Whenever this creature attacks and isn't blocked, you may put the top creature card of defending player's graveyard onto the battlefield under your control. If you do, this creature assigns no combat damage this turn.
//! Set: WTH #62 — Weatherlight | Scryfall ID: 207bb4cd-4525-47e0-b412-0d0e29717d44 | Oracle ID: 13b5c9ef-d0e3-453d-ae5e-8de4f310cc38
// PARTIAL: a 2/2 body only; the attack trigger is left out, because no effect puts the top creature card of defending player's graveyard onto the battlefield, and the "assigns no combat damage" half alone would be a drawback offered for nothing.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BONE_DANCER,
    oracle_id = "13b5c9ef-d0e3-453d-ae5e-8de4f310cc38",
    scryfall_id = "207bb4cd-4525-47e0-b412-0d0e29717d44",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "a 2/2 body only; the attack trigger is left out, because no effect puts the top creature card of defending player's graveyard onto the battlefield, and the \"assigns no combat damage\" half alone would be a drawback offered for nothing",
    ),
    faces = &[face!(
        name = "Bone Dancer",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: "Whenever this creature attacks and isn't blocked, you may put the top
    // creature card of defending player's graveyard onto the battlefield under your control. If
    // you do, this creature assigns no combat damage this turn." No effect picks the top creature
    // card of a graveyard and puts it onto the battlefield under your control; the old trigger
    // offered only "assign no combat damage", a self-harming option that gained nothing.
);
