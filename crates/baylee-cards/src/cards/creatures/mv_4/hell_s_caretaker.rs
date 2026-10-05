//! Hell's Caretaker — {3}{B} — Creature — Horror
//! Oracle: {T}, Sacrifice a creature: Return target creature card from your graveyard to the battlefield. Activate only during your upkeep.
//! Set: A25 #92 — Masters 25 | Scryfall ID: 84a65ccf-37b6-48a3-9873-7a4cafef27cb | Oracle ID: 2f0d797e-d897-453d-92b6-a90e1a548dc5
// IMPLEMENTED — {T}, sacrifice a creature: return a target creature card from your graveyard to the battlefield, only during your upkeep.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HELL_S_CARETAKER,
    oracle_id = "2f0d797e-d897-453d-92b6-a90e1a548dc5",
    scryfall_id = "84a65ccf-37b6-48a3-9873-7a4cafef27cb",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hell's Caretaker",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(TapSelf, Sacrifice(&Filter::CREATURE)),
        &[Effect::reanimate(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You
        ))],
        target = Some(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You
        )),
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::Upkeep)
        ])),
    )],
);
