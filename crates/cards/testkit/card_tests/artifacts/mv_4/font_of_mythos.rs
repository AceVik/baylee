//! `cards/artifacts/mv_4/font_of_mythos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn font_of_mythos() -> CardIndex {
    card_index("7194a262-5e7a-4c12-b271-2bc5e0799477")
}

/// "At the beginning of each player's draw step, that player draws two
/// additional cards." The opponent's step is the half a "you" would get wrong:
/// each seat's library drops by the normal draw plus two on its own turn.
#[test]
fn each_player_draws_two_extra_in_their_own_draw_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[font_of_mythos()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let (a0, a1) = (library_size(&engine, p0), library_size(&engine, p1));
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        library_size(&engine, p1),
        a1 - 3,
        "the opponent's draw step: one for the turn and two from the Font"
    );
    assert_eq!(
        library_size(&engine, p0),
        a0,
        "and nothing is drawn by the Font's controller on that step"
    );

    let b0 = library_size(&engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3 && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(
        library_size(&engine, p0),
        b0 - 3,
        "the Font's controller draws two extra on their own turn too"
    );
}
