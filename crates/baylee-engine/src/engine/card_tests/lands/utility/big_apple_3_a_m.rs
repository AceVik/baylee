//! `cards/lands/utility/big_apple_3_a_m.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Big Apple, 3 a.m. prints `This land enters tapped. As it enters, choose a color`, `{T}: Add one
/// mana of the chosen color`, and `{5}, {T}: Create a 1/1 black Rat creature token for each opponent you have.`
/// The card is marked `Coverage::Partial` because creating tokens per opponent is not expressible in `Amount`.
/// When played, Big Apple, 3 a.m. prompts for a color as it enters, arrives tapped, untaps on the next turn,
/// and taps for exactly the chosen color while omitting the token ability.
#[test]
fn big_apple_chooses_color_enters_tapped_and_taps_for_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[big_apple_3_a_m()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let played = play_land(&mut engine, p0, big_apple_3_a_m());

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice upon entry");
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(entered_tapped(&engine, played));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, played));

    activate(&mut engine, p0, big_apple_3_a_m(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, played));
}
