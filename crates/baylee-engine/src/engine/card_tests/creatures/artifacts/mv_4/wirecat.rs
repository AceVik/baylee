#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn wirecat() -> CardIndex {
    card_index("57828973-df3d-4288-988d-147a3016120e")
}

fn veil_of_birds() -> CardIndex {
    card_index("fc6278e5-7eb1-4590-880e-fe9efa0ec4d9")
}

#[test]
fn wirecat_gains_cant_attack_and_cant_block_when_enchantment_on_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[veil_of_birds()])
        .battlefield(0, &[wirecat(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cat = on_battlefield(&engine, p0, wirecat()).expect("wirecat on bf");
    assert!(!keywords(&engine, cat).contains(KeywordSet::CANT_ATTACK));

    cast_from_hand(&mut engine, p0, veil_of_birds());
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, cat).contains(KeywordSet::CANT_ATTACK));
    assert!(keywords(&engine, cat).contains(KeywordSet::CANT_BLOCK));
}

#[test]
fn wirecat_attack_offer_changes_when_an_enchantment_resolves() {
    let p0 = PlayerId::new(0);
    for enchanted in [false, true] {
        let mut engine = lk::game(SEED, &[wirecat()], &[], &[veil_of_birds()], &[]);
        if enchanted {
            lk::float(&mut engine, p0, &[(ManaColor::Blue, 1)]);
            lk::cast_at(&mut engine, p0, veil_of_birds(), &[], &[]);
        }
        let cat = lk::obj(&engine, p0, wirecat());
        assert_eq!(
            lk::attackers_offered(&mut engine).contains(&cat),
            !enchanted
        );
    }
}

#[test]
fn wirecat_block_offer_reads_enchantments_on_either_side() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for enchanted in [false, true] {
        let board = if enchanted {
            vec![gray_ogre(), veil_of_birds()]
        } else {
            vec![gray_ogre()]
        };
        let mut engine = lk::game(SEED, &[wirecat()], &board, &[], &[]);
        reach_their_main_phase(&mut engine, p1);
        let cat = lk::obj(&engine, p0, wirecat());
        let ogre = lk::obj(&engine, p1, gray_ogre());
        let offered = lk::attack(&mut engine, &[ogre]);
        assert_eq!(lk::may_block(&offered, cat, ogre), !enchanted);
    }
}
