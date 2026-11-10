//! `cards/creatures/mv_7/kazarov_sengir_pureblood.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn kazarov_sengir_pureblood() -> CardIndex {
    card_index("3067c653-ac29-4f1d-822f-7e2e5bb38a62")
}

/// Kazarov, Sengir Pureblood — a 4/4 flier for {5}{B}{B}.
#[test]
fn kazarov_costs_seven_and_is_a_four_four_flier() {
    let mut engine = table7(&lots(swamp(), 7), &[kazarov_sengir_pureblood()], &[]);
    let kazarov = cast_and_resolve(&mut engine, kazarov_sengir_pureblood());
    assert_body(&engine, kazarov, (4, 4), KeywordSet::FLYING);
}

/// "{3}{R}: Kazarov deals 2 damage to target creature." and "Whenever a
/// creature an opponent controls is dealt damage, put a +1/+1 counter on
/// Kazarov." Shooting my own Ogre leaves Kazarov alone; shooting theirs puts
/// one counter on him.
#[test]
fn kazarov_grows_when_an_opposing_creature_is_dealt_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = lots(mountain(), 8);
    board.extend_from_slice(&[kazarov_sengir_pureblood(), gray_ogre()]);
    let mut engine = table7(&board, &[], &[gray_ogre()]);
    let kazarov = on_battlefield(&engine, p0, kazarov_sengir_pureblood()).expect("out");
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("out");
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, kazarov_sengir_pureblood(), 1);
    let _ = aim_at(&mut engine, p0, mine);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, gray_ogre()).is_some(),
        "2 kills it"
    );
    assert_eq!(
        counters_on(&engine, kazarov, CounterKind::P1P1),
        0,
        "my own creature's damage is not an opponent's"
    );

    activate(&mut engine, p0, kazarov_sengir_pureblood(), 1);
    let _ = aim_at(&mut engine, p0, theirs);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert_eq!(counters_on(&engine, kazarov, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, kazarov), (5, 5));
}
