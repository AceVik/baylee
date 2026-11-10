//! Coral Fighters — {1}{U} — Creature — Merfolk Soldier
//! Oracle: Whenever this creature attacks and isn't blocked, look at the top card of defending player's library. You may put that card on the bottom of that player's library.
//! Set: DDI #4 — Duel Decks: Venser vs. Koth | Scryfall ID: 4f6e1cf2-1c0e-4b9b-9687-dd02ac019fbc | Oracle ID: 8a397a64-542f-43fb-bf54-27ac651e1819
// NOT SUPPORTED: Optionally putting the top card of defending player's library on the bottom of their library after looking at it (`LookAtTopOf` does not offer a top-or-bottom fateseal choice on another player's library).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CORAL_FIGHTERS,
    oracle_id = "8a397a64-542f-43fb-bf54-27ac651e1819",
    scryfall_id = "4f6e1cf2-1c0e-4b9b-9687-dd02ac019fbc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Optionally putting the top card of defending player's library on the bottom of their library after looking at it is not yet supported (`LookAtTopOf` does not offer a top-or-bottom fateseal choice on another player's library); in play, when Coral Fighters attacks and isn't blocked, you look at the top card of defending player's library.",
    ),
    faces = &[face!(
        name = "Coral Fighters",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::LookAtTopOf {
            player: PlayerRel::DefendingPlayer,
            count: 1,
        }],
    )],
);
