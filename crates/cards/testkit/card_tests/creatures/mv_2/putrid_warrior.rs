//! `cards/creatures/mv_2/putrid_warrior.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn putrid_warrior() -> CardIndex {
    card_index("6ec24c24-7e87-4c12-b94e-3cd5b184beb5")
}

/// Putrid Warrior ({W}{B} 2/2 Zombie Soldier Warrior):
/// "Whenever this creature deals damage, choose one —
/// • Each player loses 1 life.
/// • Each player gains 1 life."
#[test]
fn putrid_warrior_triggers_on_player_or_creature_damage_for_chosen_symmetric_life_mode() {
    // Mode 0 on unblocked combat damage to player: P1 takes 2 combat -> 18, then both lose 1 -> 19 / 17.
    let mut e = lk::game(SEED, &[putrid_warrior()], &[], &[], &[]);
    let warrior = lk::obj(&e, lk::P0, putrid_warrior());

    let _ = lk::attack(&mut e, &[warrior]);
    lk::block(&mut e, &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let lose_mode = choose_cast_kind(&e, CastModeKind::Mode(0));
    e.apply(lk::P0, PlayerAction::ChooseMode(lose_mode))
        .unwrap();
    lk::through_combat(&mut e);

    assert_eq!(lk::life(&e, lk::P0), 19);
    assert_eq!(lk::life(&e, lk::P1), 17);

    // Mode 1 on combat damage to a blocker: still triggers "deals damage", both gain 1 -> 21 / 21.
    let mut e2 = lk::game(SEED, &[putrid_warrior()], &[pearled_unicorn()], &[], &[]);
    let warrior2 = lk::obj(&e2, lk::P0, putrid_warrior());
    let unicorn = lk::obj(&e2, lk::P1, pearled_unicorn());

    let _ = lk::attack(&mut e2, &[warrior2]);
    lk::block(&mut e2, &[(unicorn, warrior2)]);
    pass_until(&mut e2, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let gain_mode = choose_cast_kind(&e2, CastModeKind::Mode(1));
    e2.apply(lk::P0, PlayerAction::ChooseMode(gain_mode))
        .unwrap();
    lk::through_combat(&mut e2);

    assert_eq!(lk::life(&e2, lk::P0), 21);
    assert_eq!(lk::life(&e2, lk::P1), 21);
}
