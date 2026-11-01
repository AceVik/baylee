//! `cards/creatures/mv_3/death_s_head_buzzard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn death_s_head_buzzard_gives_all_creatures_minus_one_minus_one_when_it_dies() {
    let buzzard = card_index("0dd73855-6a7a-4eb4-80e0-1509f9397e5a");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[buzzard, mountain()])
        .battlefield(1, &[index::LLANOWAR_ELVES, index::GRIZZLY_BEARS])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let buzzard_id = on_battlefield(&engine, p0, buzzard).unwrap();
    assert!(keywords(&engine, buzzard_id).contains(KeywordSet::FLYING));
    let bears_id = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::LIGHTNING_BOLT);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![buzzard_id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, buzzard).is_some());
    assert!(
        in_graveyard(&engine, p1, index::LLANOWAR_ELVES).is_some(),
        "1/1 Llanowar Elves dies from the -1/-1 trigger"
    );
    assert_eq!(pt(&engine, bears_id), (1, 1));
}
