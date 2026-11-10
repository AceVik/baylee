//! Chainer, Dementia Master — {3}{B}{B} — Legendary Creature — Human Minion
//! Oracle: All Nightmares get +1/+1.
//! Oracle: {B}{B}{B}, Pay 3 life: Put target creature card from a graveyard onto the battlefield under your control. That creature is black and is a Nightmare in addition to its other creature types.
//! Oracle: When Chainer leaves the battlefield, exile all Nightmares.
//! Set: CMM #144 — Commander Masters | Scryfall ID: 186c30e6-2b84-415d-83e4-ecddaeb11ca1 | Oracle ID: cb905eb0-bb20-41c0-9695-0f5c71972561
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHAINER_DEMENTIA_MASTER,
    oracle_id = "cb905eb0-bb20-41c0-9695-0f5c71972561",
    scryfall_id = "186c30e6-2b84-415d-83e4-ecddaeb11ca1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Chainer, Dementia Master",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::MINION],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
