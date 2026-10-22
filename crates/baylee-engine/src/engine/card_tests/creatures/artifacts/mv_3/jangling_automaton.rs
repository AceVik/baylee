//! `cards/creatures/artifacts/mv_3/jangling_automaton.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, JANGLING_AUTOMATON};

/// `Jangling Automaton` ({3}, Artifact Creature — Construct 3/2) untaps all
/// creatures defending player controls whenever it attacks.
#[test]
fn jangling_automaton_untaps_defenders_creatures_on_attack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[JANGLING_AUTOMATON])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let automaton = on_battlefield(&engine, p0, JANGLING_AUTOMATON).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    engine
        .state
        .object_mut(bears)
        .unwrap()
        .status
        .insert(crate::object::Status::TAPPED);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(automaton, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, bears));
}
