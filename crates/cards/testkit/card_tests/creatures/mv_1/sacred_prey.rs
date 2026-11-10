//! `cards/creatures/mv_1/sacred_prey.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "e24110e8-72cf-4dd0-bca0-665758daca88"
fn sacred_prey() -> CardIndex {
    card_index("e24110e8-72cf-4dd0-bca0-665758daca88")
}

/// `Sacred Prey` (`Coverage::Generated`):
/// "Whenever this creature becomes blocked, you gain 1 life."
///
/// Verifies that when `Sacred Prey` attacks and becomes blocked, its controller
/// gains 1 life.
#[test]
fn sacred_prey_gains_one_life_when_it_becomes_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9606, forest())
        .battlefield(0, &[sacred_prey()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let prey = on_battlefield(&engine, p0, sacred_prey()).expect("P0 Sacred Prey");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(prey, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, prey)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        21,
        "Sacred Prey's controller gains 1 life when it becomes blocked"
    );
}
