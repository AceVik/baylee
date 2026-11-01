//! `cards/creatures/mv_2/dauthi_warlord.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn dauthi_warlord() -> CardIndex {
    card_index("42bb6537-fa54-43b7-84f2-dea41adc35fd")
}

fn dauthi_cutthroat() -> CardIndex {
    card_index("5fdde593-54cb-4186-974b-df4383f7efe2")
}

/// Dauthi Warlord (`42bb6537-fa54-43b7-84f2-dea41adc35fd`) — {1}{B} Creature — Dauthi Soldier (Implemented):
/// "Shadow. Dauthi Warlord's power is equal to the number of creatures with shadow on the battlefield."
#[test]
fn dauthi_warlord_power_equals_number_of_creatures_with_shadow_on_battlefield() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[dauthi_warlord(), swamp(), swamp(), mountain()])
        .hand(0, &[dauthi_cutthroat(), lightning_bolt()])
        .battlefield(1, &[dauthi_cutthroat()])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    let warlord = on_battlefield(&engine, P0, dauthi_warlord()).unwrap();
    assert_eq!(pt(&engine, warlord), (2, 1));
    cast_from_hand(&mut engine, P0, dauthi_cutthroat());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, warlord), (3, 1));
    let victim = on_battlefield(&engine, PlayerId::new(1), dauthi_cutthroat()).unwrap();
    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, victim);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, warlord), (2, 1));
}

#[test]
fn shadow_limits_both_blocking_and_being_blocked() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = card_index("42bb6537-fa54-43b7-84f2-dea41adc35fd");
    let mut e = lk::game(
        SEED,
        &[subject, gray_ogre()],
        &[subject, grizzly_bears()],
        &[],
        &[],
    );
    let shadow = lk::obj(&e, p0, subject);
    let normal = lk::obj(&e, p0, gray_ogre());
    let enemy_shadow = lk::obj(&e, p1, subject);
    let enemy_normal = lk::obj(&e, p1, grizzly_bears());
    let offer = lk::attack(&mut e, &[shadow, normal]);
    assert!(lk::may_block(&offer, enemy_shadow, shadow));
    assert!(lk::may_block(&offer, enemy_normal, normal));
    assert!(!lk::may_block(&offer, enemy_normal, shadow));
    assert!(!lk::may_block(&offer, enemy_shadow, normal));
}
