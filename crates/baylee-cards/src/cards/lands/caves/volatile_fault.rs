//! Volatile Fault — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice this land: Destroy target nonbasic land an opponent controls. That player may search their library for a basic land card, put it onto the battlefield, then shuffle. You create a Treasure token.
//! Set: LCI #286 — The Lost Caverns of Ixalan | Scryfall ID: 9385abf3-b067-4586-bf3d-175526cf8f0a | Oracle ID: 95c44f28-f7fa-4785-83b9-0d81be0db0c8
// PARTIAL — {C} always; {1}, {T}, Sacrifice this land destroys a target
// nonbasic land an opponent controls, lets that player search up a basic land
// and creates a Treasure. The searched land enters tapped where the card
// prints untapped.

use crate::tokens::TREASURE;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Target nonbasic land an opponent controls" — the ability's own target
/// requirement and the thing its destroy points at, so one static spells both.
static OPPONENT_NONBASIC_LAND: Filter = f!(opponents NONBASIC_LAND);

card!(
    index = index::VOLATILE_FAULT,
    oracle_id = "95c44f28-f7fa-4785-83b9-0d81be0db0c8",
    scryfall_id = "9385abf3-b067-4586-bf3d-175526cf8f0a",
    faces = &[face!(
        name = "Volatile Fault",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
    ),],
    coverage = Coverage::Partial(
        "the searched basic land enters tapped, not untapped — the DSL's only \
         search into another player's library is OptionalBasicLandSearchFor, \
         which puts it onto the battlefield tapped (Path to Exile's wording)",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "put it onto the battlefield" (untapped) —
        // `Effect::OptionalBasicLandSearchFor` reaches that player's library
        // and finds the basic land, but its destination is fixed at the
        // battlefield *tapped*. Dropping the search instead would be worse
        // than the tap: the sentence exists to give the land back.
        activated!(
            cost!("{1}", TapSelf, SacrificeSelf),
            &[
                Effect::destroy(TargetSpec::Object(&OPPONENT_NONBASIC_LAND)),
                Effect::OptionalBasicLandSearchFor {
                    player: PlayerRel::ControllerOfTarget,
                },
                Effect::CreateToken { token: &TREASURE },
            ],
            target = Some(TargetSpec::Object(&OPPONENT_NONBASIC_LAND)),
        ),
    ],
);
