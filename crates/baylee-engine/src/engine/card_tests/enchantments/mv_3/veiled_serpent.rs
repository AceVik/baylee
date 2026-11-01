#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn veiled_serpent() -> CardIndex {
    card_index("ff59a95c-28de-44cf-abbe-772417851ffa")
}

#[test]
fn veiled_serpent_becomes_serpent_creature_when_opponent_casts_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[veiled_serpent()])
        .hand(1, &[gray_ogre()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, gray_ogre());
    pass_until(&mut engine, stack_is_empty);

    let serpent = on_battlefield(&engine, p0, veiled_serpent()).expect("serpent on bf");
    assert_eq!(pt(&engine, serpent), (4, 4));
}

#[test]
fn veiled_serpent_attacks_only_a_defender_with_an_island() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for land in [island(), mountain()] {
        let mut engine = lk::game(SEED, &[veiled_serpent()], &[land], &[], &[gray_ogre()]);
        reach_their_main_phase(&mut engine, p1);
        lk::float(&mut engine, p1, &[(ManaColor::Red, 3)]);
        lk::cast_at(&mut engine, p1, gray_ogre(), &[], &[]);
        reach_their_main_phase(&mut engine, p0);
        let serpent = lk::obj(&engine, p0, veiled_serpent());
        assert_eq!(
            lk::attackers_offered(&mut engine).contains(&serpent),
            land == island()
        );
    }
}

#[test]
fn veiled_serpent_cycles_from_hand_for_two() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(SEED, &[], &[], &[veiled_serpent()], &[]);
    let card = in_hand(&engine, p0, veiled_serpent()).unwrap();
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    lk::float(&mut engine, p0, &[(ManaColor::Colorless, 2)]);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: card,
                ability_index: 1,
            },
        )
        .unwrap();
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, p0, veiled_serpent()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before
    );
    assert_eq!(lk::pool_total(&engine, p0), 0);
}
