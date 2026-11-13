//! Camouflage — {G} — Instant
//! Oracle: Cast this spell only during your declare attackers step.
//! Oracle: This turn, instead of declaring blockers, each defending player chooses any number of creatures they control and divides them into a number of piles equal to the number of attacking creatures for whom that player is the defending player. Creatures those players control that can block additional creatures may likewise be put into additional piles. Assign each pile to a different one of those attacking creatures at random. Each creature in a pile that can block the creature that pile is assigned to does so. (Piles can be empty.)
//! Set: 2ED #188 — Unlimited Edition | Scryfall ID: 09243dc6-c56c-42a8-969b-2ecffe89e1ca | Oracle ID: 9cf44db4-627a-4197-9588-6da72e41f03d
// IMPLEMENTED — cast in its controller's declare attackers step; this turn
// each defending player divides creatures into piles, assigned to the
// attackers at random by the game's seeded generator, and each creature that
// can block its pile's attacker does.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CAMOUFLAGE,
    oracle_id = "9cf44db4-627a-4197-9588-6da72e41f03d",
    scryfall_id = "09243dc6-c56c-42a8-969b-2ecffe89e1ca",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Camouflage",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::BlockInPilesAtRandomThisTurn],
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::DeclareAttackers),
        ]))
    )],
);
