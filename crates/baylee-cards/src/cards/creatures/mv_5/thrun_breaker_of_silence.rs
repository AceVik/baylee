//! Thrun, Breaker of Silence — {3}{G}{G} — Legendary Creature — Troll Shaman
//! Oracle: This spell can't be countered.
//! Oracle: Trample
//! Oracle: Thrun can't be the target of nongreen spells your opponents control or abilities from nongreen sources your opponents control.
//! Oracle: During your turn, Thrun has indestructible.
//! Set: ONE #186 — Phyrexia: All Will Be One | Scryfall ID: 6d9f51dd-0393-4b3c-bea5-8f74634ab0e5 | Oracle ID: 789b7af5-ac15-40b6-b5b7-f3fcdcfb52e1
// PARTIAL — "can't be countered" and trample are two keyword bits the engine
// reads; the other two sentences have no DSL spelling.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "Thrun can't be the target of nongreen spells your opponents
// control or abilities from nongreen sources your opponents control." — no
// Modifier narrows targeting. `hexproof` is wider (it stops every colour) and
// `ProtectionFrom` is a different sentence: it also prevents damage and
// blocking, which this card does not print.
// NOT SUPPORTED: "During your turn, Thrun has indestructible." — a
// StaticAbility carries no Condition, so a keyword cannot be granted for one
// player's turn only; writing it unconditionally would be a strictly stronger
// card.

card!(
    index = index::THRUN_BREAKER_OF_SILENCE,
    oracle_id = "789b7af5-ac15-40b6-b5b7-f3fcdcfb52e1",
    scryfall_id = "6d9f51dd-0393-4b3c-bea5-8f74634ab0e5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Thrun, Breaker of Silence",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::TROLL, subtypes::creature::SHAMAN],
        power = Some(5),
        toughness = Some(5),
    ),],
    keywords = KeywordSet::UNCOUNTERABLE.union(KeywordSet::TRAMPLE),
    coverage = Coverage::Partial(
        "can't be the target of nongreen spells and abilities from nongreen \
         sources, and indestructible during your turn, are not expressible"
    ),
);
