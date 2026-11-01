#[allow(clippy::wildcard_imports)]
use super::*;

fn redeem() -> CardIndex {
    card_index("c2032f88-a000-4ed2-a7e5-61d29723d45e")
}

#[test]
fn redeem_prevents_all_damage_to_targeted_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[redeem(), lightning_bolt()])
        .battlefield(0, &[gray_ogre(), plains(), plains(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear_id = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    cast_from_hand(&mut engine, p0, redeem());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear_id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, bear_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
    assert_eq!(engine.state().object(bear_id).unwrap().damage, 0);
}

#[test]
fn redeem_can_protect_two_creatures_but_expires_and_does_not_protect_others() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[gray_ogre(), forest()],
        &[wild_elephant(), grizzly_bears()],
        &[
            redeem(),
            lightning_bolt(),
            lightning_bolt(),
            lightning_bolt(),
            lightning_bolt(),
        ],
        &[],
    );
    let mine = lk::obj(&e, p0, gray_ogre());
    let theirs = lk::obj(&e, p1, wild_elephant());
    let other = lk::obj(&e, p1, grizzly_bears());
    lk::float(&mut e, p0, &[(ManaColor::White, 2), (ManaColor::Red, 3)]);
    cast_with_floating(&mut e, p0, redeem());
    assert!(
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![mine, theirs, other],
                players: vec![]
            }
        )
        .is_err()
    );
    lk::aim(&mut e, &[mine, theirs], &[]);
    lk::settle(&mut e);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[mine], &[]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[theirs], &[]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[other], &[]);
    assert_eq!(lk::damage_on(&e, mine), 0);
    assert_eq!(lk::damage_on(&e, theirs), 0);
    assert!(in_graveyard(&e, p1, grizzly_bears()).is_some());
    reach_their_main_phase(&mut e, p1);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[mine], &[]);
    assert!(in_graveyard(&e, p0, gray_ogre()).is_some());
}
