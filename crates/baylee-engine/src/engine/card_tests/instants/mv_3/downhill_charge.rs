//! `cards/instants/mv_3/downhill_charge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Downhill Charge ({2}{R} Instant):
/// "Target creature gets +X/+0 until end of turn, where X is the number of Mountains you control."
#[test]
fn downhill_charge_gives_target_creature_plus_x_plus_zero_for_your_mountains() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), grizzly_bears()])
        .hand(0, &[index::DOWNHILL_CHARGE])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, index::DOWNHILL_CHARGE);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // 3 Mountains controlled by P0 -> +3/+0 (P1's Mountain is not counted).
    assert_eq!(pt(&engine, bears), (5, 2));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2));
}
