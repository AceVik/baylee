//! Rysorian Badger — {2}{G} — Creature — Badger
//! Oracle: Whenever this creature attacks and isn't blocked, you may exile up to two target creature cards from defending player's graveyard. If you do, you gain 1 life for each card exiled this way and this creature assigns no combat damage this turn.
//! Set: HML #96 — Homelands | Scryfall ID: ab87a387-678b-4913-a0c7-85f0238cee26 | Oracle ID: cf9fc317-3184-4999-84ef-945a8842096e

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const DEFENDER_GRAVEYARD_CREATURE: TargetSpec =
    TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::DefendingPlayer);

// NOT SUPPORTED: counting cards exiled from a graveyard this way is not in ThisWay, so accepting the trigger exiles the chosen graveyard targets, gains 1 life, and suppresses this creature's combat damage this turn.
card!(
    index = index::RYSORIAN_BADGER,
    oracle_id = "cf9fc317-3184-4999-84ef-945a8842096e",
    scryfall_id = "ab87a387-678b-4913-a0c7-85f0238cee26",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "counting cards exiled from defending player's graveyard this way to scale life gain and gate combat-damage suppression is not in ThisWay; exiles up to two target creature cards from defending player's graveyard, gains 1 life, and assigns no combat damage this turn when chosen"
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
        targets = Some(TargetReq::up_to(DEFENDER_GRAVEYARD_CREATURE, 2))
    )],
);
