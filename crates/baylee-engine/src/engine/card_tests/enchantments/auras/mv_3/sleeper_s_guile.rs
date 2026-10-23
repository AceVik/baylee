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
