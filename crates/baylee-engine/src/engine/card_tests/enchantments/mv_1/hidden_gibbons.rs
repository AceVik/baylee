#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn hidden_gibbons() -> CardIndex {
    card_index("2315b044-bfd2-40dd-a73e-4cf064b36999")
}

#[test]
fn hidden_gibbons_becomes_four_four_ape_when_opponent_casts_instant() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[hidden_gibbons()])
        .hand(1, &[lightning_bolt()])
        .battlefield(0, &[forest()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, hidden_gibbons());
    pass_until(&mut engine, stack_is_empty);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let gib = on_battlefield(&engine, p0, hidden_gibbons()).unwrap();
    assert_eq!(pt(&engine, gib), (4, 4));
}

#[test]
fn hidden_gibbons_ignores_own_instants_and_opponent_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[hidden_gibbons()],
        &[],
        &[lightning_bolt()],
        &[gray_ogre()],
    );
    let gibbons = lk::obj(&engine, p0, hidden_gibbons());
    lk::float(&mut engine, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, p0, lightning_bolt(), &[], &[p1]);
    assert!(
        engine
            .state()
            .object(gibbons)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    reach_their_main_phase(&mut engine, p1);
    lk::float(&mut engine, p1, &[(ManaColor::Red, 3)]);
    lk::cast_at(&mut engine, p1, gray_ogre(), &[], &[]);
    assert!(
        engine
            .state()
            .object(gibbons)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
}
