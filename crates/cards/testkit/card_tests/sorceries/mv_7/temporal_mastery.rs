//! `cards/sorceries/mv_7/temporal_mastery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Temporal Mastery: "Take an extra turn after this one. Exile Temporal
/// Mastery." Cast for its full cost, the next turn is its caster's again.
#[test]
fn temporal_mastery_takes_an_extra_turn_and_exiles_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4402, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[temporal_mastery_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let turn = engine.state().turn.number;
    cast_from_hand(&mut engine, p0, temporal_mastery_card());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(exiled(&engine, p0, temporal_mastery_card()), 1);
    assert!(in_graveyard(&engine, p0, temporal_mastery_card()).is_none());

    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert_eq!(engine.state().turn.number, turn + 1);
    assert_eq!(
        engine.state().turn.active,
        p0,
        "the extra turn is the caster's, not the opponent's"
    );
}
