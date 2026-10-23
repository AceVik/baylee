//! `cards/enchantments/auras/mv_2/sluggishness.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sluggishness() -> CardIndex {
    card_index("39eb4062-68a0-4f64-a9cc-79be87fd8173")
}

#[test]
fn sluggishness_grants_cant_block_and_returns_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), gray_ogre()])
        .hand(0, &[sluggishness(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, sluggishness());
    if let Pending::ChooseTargets { options, .. } = engine.pending() {
        assert!(options.contains(&ogre));
        for land in engine.state().zones.list(ZoneLocation::Battlefield) {
            if engine
                .state()
                .object(*land)
                .unwrap()
                .characteristics()
                .types
                .contains(baylee_core::types::TypeSet::LAND)
            {
                assert!(!options.contains(land), "enchant creature excludes lands");
            }
        }
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ogre],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, ogre).contains(baylee_cards_dsl::KeywordSet::CANT_BLOCK));

    cast_from_hand(&mut engine, p0, lightning_bolt());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ogre],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, sluggishness()).is_some());
}

#[test]
fn only_the_enchanted_creature_cannot_block() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(
        SEED,
        &[gray_ogre(), earth_elemental()],
        &[gray_ogre()],
        &[sluggishness()],
        &[],
    );
    let enchanted = lk::obj(&engine, lk::P0, gray_ogre());
    let free = lk::obj(&engine, lk::P0, earth_elemental());
    lk::float(&mut engine, lk::P0, &[(ManaColor::Red, 2)]);
    lk::cast_at(&mut engine, lk::P0, sluggishness(), &[enchanted], &[]);
    reach_their_main_phase(&mut engine, lk::P1);
    let attacker = lk::obj(&engine, lk::P1, gray_ogre());
    let offer = lk::attack(&mut engine, &[attacker]);
    assert!(!lk::may_block(&offer, enchanted, attacker));
    assert!(lk::may_block(&offer, free, attacker));
}
