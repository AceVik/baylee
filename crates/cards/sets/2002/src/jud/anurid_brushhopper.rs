//! Anurid Brushhopper — {1}{G}{W} — Creature — Frog Beast
//! Oracle: Discard two cards: Exile this creature. Return it to the battlefield under its owner's control at the beginning of the next end step.
//! Set: JUD #137 — Judgment | Scryfall ID: b09204c7-3e3d-484a-a4f7-da1b818e3884 | Oracle ID: bdecf59c-f8bd-4003-9058-685b70c08837
// IMPLEMENTED — discard-two blink returning self at the beginning of the next end step.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANURID_BRUSHHOPPER,
    oracle_id = "bdecf59c-f8bd-4003-9058-685b70c08837",
    scryfall_id = "b09204c7-3e3d-484a-a4f7-da1b818e3884",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Anurid Brushhopper",
        mana_cost = mana!("{1}{G}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FROG, subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!(Discard(&Filter::Any), Discard(&Filter::Any)),
        &[
            Effect::ExileLinked {
                target: TargetSpec::ThisObject,
                until: None,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::ReturnLinkedToBattlefield],
            },
        ],
    )],
);
