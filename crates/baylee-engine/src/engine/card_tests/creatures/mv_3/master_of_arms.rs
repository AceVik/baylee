//! `cards/creatures/mv_3/master_of_arms.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, MASTER_OF_ARMS};

/// `Master of Arms` ({2}{W}, Creature — Human Soldier 2/2) has first strike and
/// `{1}{W}`: Tap target creature blocking this creature.
#[test]
fn master_of_arms_taps_blocking_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[MASTER_OF_ARMS, plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let master = on_battlefield(&engine, p0, MASTER_OF_ARMS).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(master, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, master)],
            },
        )
        .unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, MASTER_OF_ARMS, 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, bears));
}
