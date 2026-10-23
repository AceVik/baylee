//! `cards/enchantments/auras/mv_3/sleeper_s_guile.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sleeper_s_guile() -> CardIndex {
    card_index("eaaa3e41-dbf6-43ae-b65e-e1502ba4fa0b")
}

#[test]
fn sleeper_s_guile_grants_fear_and_returns_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), mountain(), gray_ogre()])
        .hand(0, &[sleeper_s_guile(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, sleeper_s_guile());
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

    assert!(keywords(&engine, ogre).contains(baylee_cards_dsl::KeywordSet::FEAR));

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

    assert!(in_hand(&engine, p0, sleeper_s_guile()).is_some());
}

#[test]
fn fear_allows_black_and_artifact_blockers_only() {
    use crate::engine::card_tests::legends_kit as lk;
    let artifact_creature = card_index("963de959-848d-48df-90fa-88a6d988b9fe");
    let mut engine = lk::game(
        SEED,
        &[gray_ogre()],
        &[llanowar_elves(), festering_goblin(), artifact_creature],
        &[sleeper_s_guile()],
        &[],
    );
    let attacker = lk::obj(&engine, lk::P0, gray_ogre());
    let green = lk::obj(&engine, lk::P1, llanowar_elves());
    let black = lk::obj(&engine, lk::P1, festering_goblin());
    let artifact = lk::obj(&engine, lk::P1, artifact_creature);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Black, 3)]);
    lk::cast_at(&mut engine, lk::P0, sleeper_s_guile(), &[attacker], &[]);
    let offer = lk::attack(&mut engine, &[attacker]);
    assert!(!lk::may_block(&offer, green, attacker));
    assert!(lk::may_block(&offer, black, attacker));
    assert!(lk::may_block(&offer, artifact, attacker));
}
