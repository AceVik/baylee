//! Crovax the Cursed — {2}{B}{B} — Legendary Creature — Vampire Noble
//! Oracle: Crovax enters with four +1/+1 counters on it.
//! Oracle: At the beginning of your upkeep, you may sacrifice a creature. If you do, put a +1/+1 counter on Crovax. If you don't, remove a +1/+1 counter from Crovax.
//! Oracle: {B}: Crovax gains flying until end of turn.
//! Set: TPR #90 — Tempest Remastered | Scryfall ID: 487e843f-b710-452c-93fe-fce3f90a4d1c | Oracle ID: e1b74ef7-ab6b-4d40-8087-3f3a448b7e0a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CROVAX_THE_CURSED,
    oracle_id = "e1b74ef7-ab6b-4d40-8087-3f3a448b7e0a",
    scryfall_id = "487e843f-b710-452c-93fe-fce3f90a4d1c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Crovax the Cursed",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::VAMPIRE, subtypes::creature::NOBLE],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
