//! `cards/creatures/mv_8/eldrazi_devastator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn eldrazi_devastator() -> CardIndex {
    card_index("bc08ac66-969c-4905-a2d6-16ff565be475")
}

/// Eldrazi Devastator is an 8/9 trampler for {8}: eight Forests pay the
/// generic cost.
#[test]
fn eldrazi_devastator_is_cast_for_eight_with_trample() {
    let mut engine = at_main(&[forest(); 8], &[eldrazi_devastator()]);
    let devastator = cast_and_settle(&mut engine, eldrazi_devastator());
    assert_eq!(pt(&engine, devastator), (8, 9));
    assert!(keywords(&engine, devastator).contains(KeywordSet::TRAMPLE));
}

/// Trample: blocked by a 2/2, the 8-power Devastator assigns the blocker
/// lethal damage and the rest to the defending player.
#[test]
fn eldrazi_devastator_tramples_over_a_chump_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[eldrazi_devastator()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let devastator = on_battlefield(&engine, p0, eldrazi_devastator()).expect("it is out");
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("the bear is out");
    let blocks = attack_player(&mut engine, devastator);
    assert!(blocks.iter().any(|b| b.blocker == bear));
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, devastator)],
            },
        )
        .expect("the bear may block");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "the bear took lethal damage"
    );
    assert_eq!(
        life_of(&engine, 1),
        20 - 6,
        "8 power minus the 2 the bear needed: 6 tramples through"
    );
}
