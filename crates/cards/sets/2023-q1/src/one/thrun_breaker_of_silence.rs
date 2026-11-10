//! Thrun, Breaker of Silence — {3}{G}{G} — Legendary Creature — Troll Shaman
//! Oracle: This spell can't be countered.
//! Oracle: Trample
//! Oracle: Thrun can't be the target of nongreen spells your opponents control or abilities from nongreen sources your opponents control.
//! Oracle: During your turn, Thrun has indestructible.
//! Set: ONE #186 — Phyrexia: All Will Be One | Scryfall ID: 6d9f51dd-0393-4b3c-bea5-8f74634ab0e5 | Oracle ID: 789b7af5-ac15-40b6-b5b7-f3fcdcfb52e1
// IMPLEMENTED — uncounterable, trample, the nongreen targeting restriction
// and indestructible during its controller's turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "nongreen spells your opponents control or abilities from nongreen
/// sources your opponents control": the filter is asked of the spell or of
/// the ability's source, so one filter says both halves.
static NONGREEN_OPPONENTS: Filter = f!(opponents Filter::Not(&Filter::HasColor(
    ColorSet::from_slice(&[Color::Green])
)));

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
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::CantBeTargetedBy(&NONGREEN_OPPONENTS)
        ),
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE),
            condition = Some(Condition::YourTurn)
        ),
    ],
    coverage = Coverage::Implemented,
);
