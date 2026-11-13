//! Reincarnation — {1}{G}{G} — Instant
//! Oracle: Choose target creature. When that creature dies this turn, return a creature card from its owner's graveyard to the battlefield under the control of that creature's owner.
//! Set: C13 #166 — Commander 2013 | Scryfall ID: 7ee379cd-b3fb-487b-846c-eab02902de79 | Oracle ID: d6bf5e22-8d33-43a9-8824-435068e0a87a
// IMPLEMENTED — targets a creature and arms a delayed trigger (WhenTargetDiesThisTurn):
// when it dies this turn, you return a creature card from its owner's graveyard
// to the battlefield under that owner's control.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REINCARNATION,
    oracle_id = "d6bf5e22-8d33-43a9-8824-435068e0a87a",
    scryfall_id = "7ee379cd-b3fb-487b-846c-eab02902de79",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reincarnation",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::WhenTargetDiesThisTurn {
            effects: &[Effect::ReturnFromGraveyardOf {
                whose: PlayerRel::OwnerOfEvent,
                filter: &Filter::CREATURE,
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
