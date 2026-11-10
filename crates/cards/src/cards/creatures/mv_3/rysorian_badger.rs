//! Rysorian Badger — {2}{G} — Creature — Badger
//! Oracle: Whenever this creature attacks and isn't blocked, you may exile up to two target creature cards from defending player's graveyard. If you do, you gain 1 life for each card exiled this way and this creature assigns no combat damage this turn.
//! Set: HML #96 — Homelands | Scryfall ID: ab87a387-678b-4913-a0c7-85f0238cee26 | Oracle ID: cf9fc317-3184-4999-84ef-945a8842096e

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const DEFENDER_GRAVEYARD_CREATURE: TargetSpec =
    TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::DefendingPlayer);

// NOT SUPPORTED: "you gain 1 life for each card exiled this way". `ThisWay` has no count of cards exiled from a graveyard, so the life gain is a fixed 1: it is right for one card and 1 short for two.
// "Up to two" is written as one or two targets: with none the printed ability exiles nothing and, with no card exiled, "if you do" does not apply, so the trigger is simply not put on the stack.
card!(
    index = index::RYSORIAN_BADGER,
    oracle_id = "cf9fc317-3184-4999-84ef-945a8842096e",
    scryfall_id = "ab87a387-678b-4913-a0c7-85f0238cee26",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "'1 life for each card exiled this way' has no count of cards exiled from a graveyard in ThisWay: the life gain is a fixed 1, so it gains 1 too little when two cards are exiled; the trigger needs at least one creature card to target (printed 'up to two' also allows none, which exiles nothing)"
    ),
    faces = &[face!(
        name = "Rysorian Badger",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BADGER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[
                Effect::Exile {
                    target: DEFENDER_GRAVEYARD_CREATURE,
                },
                Effect::gain_life(1),
                Effect::ModifySource {
                    modifier: Modifier::AssignsNoCombatDamage,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
        }],
        targets = Some(TargetReq {
            spec: DEFENDER_GRAVEYARD_CREATURE,
            min: 1,
            max: 2,
            count_is_x: false,
            distinct_mana_values: false,
            chooser: baylee_cards_dsl::TargetChooser::Controller,
        })
    )],
);
