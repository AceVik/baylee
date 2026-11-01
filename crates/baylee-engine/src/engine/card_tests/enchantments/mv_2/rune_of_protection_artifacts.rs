#[allow(clippy::wildcard_imports)]
use super::*;

fn rune_of_protection_artifacts() -> CardIndex {
    card_index("469c4b77-733e-472b-aeae-6015b409beb2")
}

fn rod_of_ruin() -> CardIndex {
    card_index("c29a04ab-4e14-45d8-a993-3fffd0e2f6eb")
}

#[test]
fn rune_of_protection_artifacts_prevents_next_damage_from_chosen_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                rune_of_protection_artifacts(),
                rod_of_ruin(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rod_id = on_battlefield(&engine, p0, rod_of_ruin()).expect("rod on bf");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rune_of_protection_artifacts(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    let land = on_battlefield(&engine, p0, plains()).unwrap();
    assert!(!options.iter().any(|s| s.object == land));
    let source = *options.iter().find(|s| s.object == rod_id).unwrap();
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
    activate(&mut engine, p0, rod_of_ruin(), 0);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p0],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn rune_of_protection_artifacts_cycles_from_hand_for_two_and_draws_one() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let mut e = lk::game(SEED, &[], &[], &[rune_of_protection_artifacts()], &[]);
    lk::float(&mut e, p0, &[(ManaColor::Colorless, 2)]);
    let before = e
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .len();
    activate(&mut e, p0, rune_of_protection_artifacts(), 1);
    assert!(in_graveyard(&e, p0, rune_of_protection_artifacts()).is_some());
    lk::settle(&mut e);
    assert_eq!(
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Library(p0))
            .len(),
        before - 1
    );
    assert_eq!(
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        1
    );
    assert_eq!(lk::pool_total(&e, p0), 0);
    assert!(e.state().journal.entries().iter().any(|entry| matches!(entry.event, crate::event::GameEvent::Cycled { player, .. } if player == p0)));
}
