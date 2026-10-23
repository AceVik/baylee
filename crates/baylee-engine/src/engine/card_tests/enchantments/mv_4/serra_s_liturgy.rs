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

#[test]
fn serra_s_liturgy_partial_never_uses_a_target_without_a_verse_counter() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let mut e = lk::game_with(
        SEED,
        &[serra_s_liturgy()],
        &[forest(), sol_ring()],
        &[],
        &[],
        |e| lk::yes_no(e, false),
    );
    lk::float(&mut e, p0, &[(ManaColor::White, 1)]);
    assert!(!lk::offered(&e, serra_s_liturgy(), p0, 1));
    let id = lk::obj(&e, p0, serra_s_liturgy());
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(id)
        .unwrap()
        .counters
        .add(baylee_cards_dsl::counters::VERSE, 1);
    e.refresh_offer();
    assert!(lk::offered(&e, serra_s_liturgy(), p0, 1));
    activate(&mut e, p0, serra_s_liturgy(), 1);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target")
    };
    let excluded = lk::obj(&e, PlayerId::new(1), forest());
    assert!(!options.contains(&excluded));
    lk::aim(&mut e, &[], &[]);
    lk::settle(&mut e);
    assert!(
        in_graveyard(&e, p0, serra_s_liturgy()).is_some(),
        "up to one includes zero targets"
    );
}
