//! Festival — {W} — Instant
//! Oracle: Cast this spell only during an opponent's upkeep.
//! Oracle: Creatures can't attack this turn.
//! Set: DRK #8 — The Dark | Scryfall ID: e9357990-701a-4336-b545-ac5a24d89cad | Oracle ID: 8e594801-4a8d-4dcd-b328-7eb3de65cf90

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FESTIVAL,
    oracle_id = "8e594801-4a8d-4dcd-b328-7eb3de65cf90",
    scryfall_id = "e9357990-701a-4336-b545-ac5a24d89cad",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Festival",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
        abilities = &[AbilityDef::Spell {
            effects: &[Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::CREATURE,
                modifier: Modifier::CantAttack,
                duration: Duration::UntilEndOfTurn,
            }],
            targets: None,
            second_targets: None,
            condition: Some(Condition::All(&[
                Condition::OpponentsTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ])),
        }],
    ),],
);
