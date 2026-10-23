#[allow(clippy::wildcard_imports)]
use super::*;

fn serra_s_liturgy() -> CardIndex {
    card_index("17b7b7e4-4ae8-495d-bf0c-d18697f250ea")
}

#[test]
fn serra_s_liturgy_adds_verse_counter_and_destroys_artifact_or_enchantment() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[serra_s_liturgy(), plains()])
        .battlefield(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let liturgy_id = on_battlefield(&engine, p0, serra_s_liturgy()).expect("liturgy on bf");
    assert_eq!(
        engine
            .state()
            .object(liturgy_id)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::VERSE),
        1
    );

    let their_ring = on_battlefield(&engine, p1, sol_ring()).expect("sol ring on bf");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, serra_s_liturgy(), 1);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![their_ring],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, sol_ring()).is_none());
    assert!(in_graveyard(&engine, p1, sol_ring()).is_some());
}
