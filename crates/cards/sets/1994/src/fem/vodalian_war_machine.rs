//! Vodalian War Machine — {1}{U}{U} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Tap an untapped Merfolk you control: This creature can attack this turn as though it didn't have defender.
//! Oracle: Tap an untapped Merfolk you control: This creature gets +2/+1 until end of turn.
//! Oracle: When this creature dies, destroy all Merfolk tapped this turn to pay for its abilities.
//! Set: FEM #32 — Fallen Empires | Scryfall ID: cd962ff0-4aa6-453e-931e-bd36fc034273 | Oracle ID: f72511b0-9993-4998-b646-65a855cb8170
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const MERFOLK: Filter = Filter::HasSubtype(subtypes::creature::MERFOLK);
const YOUR_MERFOLK: Filter = Filter::And(&[Filter::ControlledByYou, MERFOLK]);
const TAPPED_MERFOLK_FOR_THIS: Filter =
    Filter::And(&[MERFOLK, Filter::TappedToPayForSourceThisTurn]);

card!(
    index = index::VODALIAN_WAR_MACHINE,
    oracle_id = "f72511b0-9993-4998-b646-65a855cb8170",
    scryfall_id = "cd962ff0-4aa6-453e-931e-bd36fc034273",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vodalian War Machine",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(4),
    ),],
    abilities = &[
        activated!(
            cost!(TapOther(&YOUR_MERFOLK)),
            &[Effect::ModifySource {
                modifier: Modifier::AttacksDespiteDefender,
                duration: Duration::UntilEndOfTurn,
            }],
        ),
        activated!(
            cost!(TapOther(&YOUR_MERFOLK)),
            &[Effect::ModifySource {
                modifier: Modifier::ModifyPT(2, 1),
                duration: Duration::UntilEndOfTurn,
            }],
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::DestroyAll {
                filter: &TAPPED_MERFOLK_FOR_THIS,
                no_regen: false,
            }],
        ),
    ],
);
