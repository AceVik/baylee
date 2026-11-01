#[allow(clippy::wildcard_imports)]
use super::*;

fn symbiosis() -> CardIndex {
    card_index("787b494b-731b-4e99-8e46-157bc86f6eea")
}

#[test]
fn symbiosis_pumps_two_target_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[symbiosis()])
        .battlefield(0, &[gray_ogre(), forest(), forest()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("elephant on bf");
    cast_from_hand(&mut engine, p0, symbiosis());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre, elephant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ogre), (4, 4));
    assert_eq!(pt(&engine, elephant), (5, 5));
}

#[test]
fn symbiosis_requires_two_distinct_creatures_and_ends_after_this_turn() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[gray_ogre(), forest()],
        &[wild_elephant()],
        &[symbiosis()],
        &[],
    );
    let ogre = lk::obj(&e, p0, gray_ogre());
    let elephant = lk::obj(&e, p1, wild_elephant());
    let land = lk::obj(&e, p0, forest());
    lk::float(&mut e, p0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, p0, symbiosis());
    assert!(
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![]
            }
        )
        .is_err()
    );
    assert!(
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre, ogre],
                players: vec![]
            }
        )
        .is_err()
    );
    assert!(
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre, land],
                players: vec![]
            }
        )
        .is_err()
    );
    lk::aim(&mut e, &[ogre, elephant], &[]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, ogre), (4, 4));
    assert_eq!(pt(&e, elephant), (5, 5));
    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, ogre), (2, 2));
    assert_eq!(pt(&e, elephant), (3, 3));
}
