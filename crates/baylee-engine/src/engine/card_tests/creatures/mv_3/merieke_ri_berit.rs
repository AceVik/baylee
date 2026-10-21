//! `cards/creatures/mv_3/merieke_ri_berit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn merieke_ri_berit() -> CardIndex {
    card_index("288fdfbb-1236-4732-afd7-e7eaba860007")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Merieke Ri Berit (288fdfbb-1236-4732-afd7-e7eaba860007) — {W}{U}{B} 1/1 Legendary Creature — Human:
/// - "Merieke Ri Berit doesn't untap during your untap step."
/// - "{T}: Gain control of target creature for as long as you control Merieke Ri Berit."
#[test]
fn merieke_ri_berit_does_not_untap_and_holds_control_of_target_creature_while_you_control_her() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[merieke_ri_berit()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let merieke = on_battlefield(&engine, P0, merieke_ri_berit()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    activate(&mut engine, P0, merieke_ri_berit(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(entered_tapped(&engine, merieke));
    assert_eq!(
        engine.state().object(bears).unwrap().controller,
        P0,
        "P0 gains control of Grizzly Bears"
    );

    // Advance through P1's turn and into P0's next main phase (turn 3):
    // Merieke Ri Berit does not untap during P0's untap step, and P0 keeps control of Grizzly Bears.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == P0
            && e.state().turn.phase == crate::turn::Phase::FirstMain
            && stack_is_empty(e)
    });
    assert!(
        entered_tapped(&engine, merieke),
        "Merieke Ri Berit does not untap during P0's untap step"
    );
    assert_eq!(
        engine.state().object(bears).unwrap().controller,
        P0,
        "P0 still controls Grizzly Bears on turn 3"
    );

    // When Merieke Ri Berit leaves P0's control (destroyed), control of Grizzly Bears ends.
    kill(&mut engine, merieke);
    assert_eq!(
        engine.state().object(bears).unwrap().controller,
        P1,
        "control of Grizzly Bears returns to P1 when Merieke Ri Berit leaves the battlefield"
    );
}
