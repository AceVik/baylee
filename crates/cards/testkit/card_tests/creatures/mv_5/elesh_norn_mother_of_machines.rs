//! `cards/creatures/mv_5/elesh_norn_mother_of_machines.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Elesh Norn, Mother of Machines: "Permanents entering don't cause
/// abilities of permanents your opponents control to trigger." The control
/// runs the same cast of Ondu Cleric with no Elesh Norn and sees its trigger
/// ask; with her it never does.
#[test]
fn elesh_norn_mother_of_machines_stops_an_opponents_enters_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let norn = card_index("5ade11c0-41dd-4b6a-9f5b-c5903a3a0d7f");
    let trigger_asked = |with_norn: bool| -> bool {
        let mut engine = Duel::new(2206, forest())
            .battlefield(0, &[if with_norn { norn } else { quiet_creature() }])
            .battlefield(1, &[plains(), plains()])
            .hand(1, &[ondu_cleric()])
            .start();
        keep_mulligans(&mut engine);
        reach_their_main_phase(&mut engine, p1);
        cast_from_hand(&mut engine, p1, ondu_cleric());
        for _ in 0..12 {
            match engine.pending().clone() {
                Pending::YesNo { .. } => return true,
                Pending::Priority { player, .. } => {
                    if on_battlefield(&engine, p1, ondu_cleric()).is_some()
                        && stack_is_empty(&engine)
                    {
                        return false;
                    }
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                other => panic!("unexpected {other:?}"),
            }
        }
        panic!("the cleric never settled")
    };
    let _ = p0;
    assert!(
        trigger_asked(false),
        "control: the trigger asks without her"
    );
    assert!(
        !trigger_asked(true),
        "with her it is never put on the stack"
    );
}
