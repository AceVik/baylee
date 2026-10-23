#[allow(clippy::wildcard_imports)]
use super::*;

fn serra_s_hymn() -> CardIndex {
    card_index("66a3ffde-4c81-4798-b99b-a6cc3a93c3e3")
}

#[test]
fn serra_s_hymn_adds_verse_counter_and_prevents_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[serra_s_hymn(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let hymn_id = on_battlefield(&engine, p0, serra_s_hymn()).expect("hymn on bf");
    assert_eq!(
        engine
            .state()
            .object(hymn_id)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::VERSE),
        1
    );

    activate(&mut engine, p0, serra_s_hymn(), 1);
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

    cast_from_hand(&mut engine, p0, lightning_bolt());
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

    assert_eq!(engine.state().players[0].life, 18);
}
