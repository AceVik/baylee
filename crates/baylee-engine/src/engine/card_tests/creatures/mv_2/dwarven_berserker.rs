//! `cards/creatures/mv_2/dwarven_berserker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DWARVEN_BERSERKER, GRIZZLY_BEARS};

/// `Dwarven Berserker` ({1}{R}, Creature — Dwarf Berserker 1/1) gets +3/+0 and
/// gains trample until end of turn whenever it becomes blocked.
#[test]
fn dwarven_berserker_gets_plus_3_plus_0_and_trample_when_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[DWARVEN_BERSERKER])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let berserker = on_battlefield(&engine, p0, DWARVEN_BERSERKER).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(berserker, Defender::Player(p1))],
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
                blockers: vec![(bears, berserker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, berserker), (4, 1));
    assert!(keywords(&engine, berserker).contains(KeywordSet::TRAMPLE));
}
