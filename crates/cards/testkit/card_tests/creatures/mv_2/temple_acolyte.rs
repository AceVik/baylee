//! `cards/creatures/mv_2/temple_acolyte.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Temple Acolyte prints `{1}{W}` for a 1/3 Human Cleric whose whole text is
/// "When this creature enters, you gain 3 life" — one body and one trigger, and
/// neither is visible from the card file. So the spell is actually cast off two
/// tapped Plains and the two readings are taken where they happen: the card
/// sitting on the stack with both life totals still at twenty (it is an
/// *enters* ability and not a cast trigger), and then the 1/3 that arrived with
/// three life on the seat that cast it. The opponent's untouched total beside
/// it is the control — "you" is the controller and not the table.
#[test]
fn temple_acolyte_enters_as_a_one_three_and_gains_three_life_for_its_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[temple_acolyte()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, temple_acolyte());
    assert!(
        on_stack(&engine, temple_acolyte()).is_some(),
        "the Acolyte is a spell before it is a creature"
    );
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (20, 20),
        "nothing has been gained yet: the printed sentence is an *enters* \
         ability, so it waits for the creature to arrive"
    );

    pass_until(&mut engine, stack_is_empty);

    let acolyte = on_battlefield(&engine, p0, temple_acolyte()).expect("the Acolyte resolved");
    assert_eq!(
        pt(&engine, acolyte),
        (1, 3),
        "the body the card prints, and the one the trigger left alone"
    );
    assert_eq!(
        engine.state().players[0].life,
        23,
        "\"you gain 3 life\" — the seat that cast it, three and not one"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nothing for the other seat, so the gain is read off the \
         controller and not off the table"
    );
}
