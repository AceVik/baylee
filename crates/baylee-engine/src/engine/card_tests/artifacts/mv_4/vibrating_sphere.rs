//! Vibrating Sphere — `{4}` Artifact (Ice Age)
//!
//! As long as it's your turn, creatures you control get +2/+0.
//! As long as it's not your turn, creatures you control get -0/-2.

#[allow(clippy::wildcard_imports)]
use super::*;

fn vibrating_sphere() -> CardIndex {
    card_index("8a8fe94f-81aa-4725-ace6-ed56a1e1ed91")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn vibrating_sphere_gives_plus_two_power_on_your_turn_and_minus_two_toughness_on_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[vibrating_sphere()])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                hill_giant(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let our_giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let our_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let opp_giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let sphere_hand = in_hand(&engine, p0, vibrating_sphere()).unwrap();

    // Cast Vibrating Sphere on p0's turn.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: sphere_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // On p0's turn: creatures p0 controls get +2/+0; opponent's creatures are unaffected.
    assert_eq!(pt(&engine, our_giant), (5, 3));
    assert_eq!(pt(&engine, our_bears), (4, 2));
    assert_eq!(
        pt(&engine, opp_giant),
        (3, 3),
        "opponent's creatures are not affected by Vibrating Sphere"
    );

    // Advance to p1's turn: creatures p0 controls get -0/-2 instead.
    // Hill Giant (3/3) becomes 3/1; Grizzly Bears (2/2) becomes 2/0 and dies to SBA!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, our_giant), (3, 1));
    assert_eq!(
        engine.state().object(our_bears).unwrap().zone,
        Zone::Graveyard,
        "2/2 Grizzly Bears gets -0/-2 on opponent's turn and dies to state-based actions"
    );
    assert_eq!(pt(&engine, opp_giant), (3, 3));
}
