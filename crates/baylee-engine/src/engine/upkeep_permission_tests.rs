//! An exiled card's permission expires at its player's upkeep boundary.
use super::synthetic::{SyntheticLookup, preset};
use super::*;
use crate::state::PlayPermission;
use crate::turn::Step;
use baylee_cards_dsl::PlayWindow;

#[test]
fn next_upkeep_permission_survives_turn_start_and_other_players_upkeep() {
    let mut engine = Engine::new(&preset(931, &[]), SyntheticLookup::new(vec![])).unwrap();
    let you = PlayerId::new(0);
    let them = PlayerId::new(1);
    let name = engine.state.names.intern("permitted card");
    let card = engine
        .state
        .create_bare(you, ObjectKind::Card, name, ZoneLocation::Exile(you));
    engine.state.play_permissions.push(PlayPermission {
        player: you,
        card,
        version: engine.state.object(card).unwrap().version,
        free: false,
        mana_any_type: false,
        cast_only: false,
        spell_types: None,
        window: PlayWindow::UntilYourNextUpkeep.into(),
    });
    engine.state.turn.active = them;
    engine.state.turn.phase = Phase::Beginning;
    engine.state.turn.step = Step::Untap;
    engine.advance_step();
    assert_eq!(engine.state.turn.step, Step::Upkeep);
    assert_eq!(
        engine.state.play_permissions.len(),
        1,
        "another player's upkeep is not yours"
    );
    engine.state.turn.active = you;
    engine.begin_turn(true);
    assert_eq!(
        engine.state.play_permissions.len(),
        1,
        "your next turn's untap still permits play"
    );
    engine.state.turn.phase = Phase::Beginning;
    engine.state.turn.step = Step::Untap;
    engine.advance_step();
    assert_eq!(engine.state.turn.step, Step::Upkeep);
    assert!(
        engine.state.play_permissions.is_empty(),
        "expired before upkeep priority"
    );
    assert!(
        engine
            .state
            .zones
            .list(ZoneLocation::Exile(you))
            .contains(&card)
    );
}
