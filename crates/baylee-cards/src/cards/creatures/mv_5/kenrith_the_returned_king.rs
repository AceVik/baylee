//! Kenrith, the Returned King — {4}{W} — Legendary Creature — Human Noble
//! Oracle: {R}: All creatures gain trample and haste until end of turn.
//! Oracle: {1}{G}: Put a +1/+1 counter on target creature.
//! Oracle: {2}{W}: Target player gains 5 life.
//! Oracle: {3}{U}: Target player draws a card.
//! Oracle: {4}{B}: Put target creature card from a graveyard onto the battlefield under its owner's control.
//! Set: PLST #ELD-303 — The List | Scryfall ID: 0e259db1-14db-4314-998c-6a076a28d8cb | Oracle ID: d209b948-9afb-4fd1-a961-72c87282878c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KENRITH_THE_RETURNED_KING,
    oracle_id = "d209b948-9afb-4fd1-a961-72c87282878c",
    scryfall_id = "0e259db1-14db-4314-998c-6a076a28d8cb",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Kenrith, the Returned King",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOBLE],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
