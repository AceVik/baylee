//! `cards/instants/mv_5/beacon_of_destruction.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn beacon_of_destruction() -> CardIndex {
    card_index("15feb59b-bbbe-462a-8d38-d20735bbc2f3")
}

/// Beacon of Destruction deals 5 damage to any target and shuffles itself into
/// its owner's library instead of going to the graveyard.
#[test]
fn beacon_of_destruction_deals_five_and_shuffles_into_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 10])
        .hand(0, &[beacon_of_destruction(), beacon_of_destruction()])
        .battlefield(1, &[index::FANGREN_HUNTER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib_before = library_size(&engine, p0);
    let lands = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| lands[..5].contains(&id));
    cast_with_floating(&mut engine, p0, beacon_of_destruction());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(lk::life(&engine, p1), 15);
    assert!(in_graveyard(&engine, p0, beacon_of_destruction()).is_none());
    assert_eq!(library_size(&engine, p0), lib_before + 1);

    let hunter = on_battlefield(&engine, p1, index::FANGREN_HUNTER).unwrap();
    tap_mana_where(&mut engine, p0, |id| lands[5..10].contains(&id));
    cast_with_floating(&mut engine, p0, beacon_of_destruction());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hunter],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p1, index::FANGREN_HUNTER).is_none());
    assert_eq!(library_size(&engine, p0), lib_before + 2);
}
