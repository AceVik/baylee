//! `cards/instants/mv_3/stench_of_decay.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn stench_of_decay() -> CardIndex {
    card_index("4d4fbb16-0421-4e9c-af9e-e2c12a11c4ed")
}

fn ornithopter() -> CardIndex {
    baylee_core::generated::index::ORNITHOPTER
}

/// Stench of Decay ({1}{B}{B} Instant):
/// - "Nonartifact creatures get -1/-1 until end of turn."
#[test]
fn stench_of_decay_gives_nonartifact_creatures_minus_one_minus_one_sparing_artifact_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), ornithopter()])
        .battlefield(1, &[grizzly_bears(), llanowar_elves()])
        .hand(0, &[stench_of_decay()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let thopter = on_battlefield(&engine, p0, ornithopter()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    tap_all_mana_but(&mut engine, p0, Some(ornithopter()));
    cast_from_hand(&mut engine, p0, stench_of_decay());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "1/1 nonartifact Llanowar Elves dies to -1/-1"
    );
    assert_eq!(
        pt(&engine, bears),
        (1, 1),
        "2/2 nonartifact Grizzly Bears becomes 1/1"
    );
    assert!(
        on_battlefield(&engine, p0, ornithopter()).is_some(),
        "0/2 artifact creature Ornithopter is unaffected"
    );
    assert_eq!(pt(&engine, thopter), (0, 2));

    // "Until end of turn": on the opponent's turn the Bears are a 2/2 again.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert_eq!(pt(&engine, bears), (2, 2));
}
