//! `cards/enchantments/mv_2/the_meathook_massacre.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Meathook Massacre: the X it announces sweeps the board, and the two
/// drain triggers read which side a creature was on.
///
/// `{X}{B}{B}` plus `Amount::NegX` in a `PumpFilter` is the sweep, and the
/// number is announced on casting (CR 601.2b) — so X is 1 here and the Elves
/// on both sides are 1/1s that CR 704.5f puts in the graveyard. Both drain
/// triggers then fire, in opposite directions, which is what separates the
/// card from one that drained on every death.
#[test]
fn the_meathook_massacre_sweeps_for_the_x_it_announced_and_drains_both_ways() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(406, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .hand(0, &[the_meathook_massacre()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, the_meathook_massacre());
    let Pending::ChooseNumber { max, .. } = engine.pending().clone() else {
        panic!("expected the X announcement, got {:?}", engine.pending())
    };
    assert!(max >= 1, "three Swamps pay {{X}}{{B}}{{B}} for X = 1");
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the sweep and both drains resolve"
    );

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none()
            && on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "-1/-1 kills a 1/1 on either side (CR 704.5f)"
    );
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (21, 19),
        "your creature dying drains them for one and theirs gains you one, so \
         the two triggers are read by which side the creature was on"
    );
}
