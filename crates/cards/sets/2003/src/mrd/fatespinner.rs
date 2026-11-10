//! Fatespinner — {1}{U}{U} — Creature — Human Wizard
//! Oracle: At the beginning of each opponent's upkeep, that player chooses draw step, main phase, or combat phase. The player skips each instance of the chosen step or phase this turn.
//! Set: MRD #36 — Mirrodin | Scryfall ID: 5780f4e0-dcfb-4d1f-8ae1-762b98970abb | Oracle ID: 676935c8-22e0-427f-9566-16778cea0873
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FATESPINNER,
    oracle_id = "676935c8-22e0-427f-9566-16778cea0873",
    scryfall_id = "5780f4e0-dcfb-4d1f-8ae1-762b98970abb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Fatespinner",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachOpponent,
        },
        &[Effect::ChoosePhaseToSkipThisTurn],
    )],
);
