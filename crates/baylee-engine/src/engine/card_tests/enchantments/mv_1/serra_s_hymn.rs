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

#[test]
fn serras_hymn_single_target_partial_requires_positive_verse_counters() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for n in [0, 2] {
        let mut e = lk::game_with(
            SEED,
            &[serra_s_hymn()],
            &[],
            &[lightning_bolt(), lightning_bolt()],
            &[],
            |e| lk::yes_no(e, false),
        );
        let hymn = lk::obj(&e, p0, serra_s_hymn());
        e.dev_state_mut(p0)
            .unwrap()
            .object_mut(hymn)
            .unwrap()
            .counters
            .add(baylee_cards_dsl::counters::VERSE, n);
        e.refresh_offer();
        if n == 0 {
            assert!(!lk::offered(&e, serra_s_hymn(), p0, 1));
            assert!(
                e.apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: hymn,
                        ability_index: 1,
                    },
                )
                .is_err(),
                "zero counters cannot allocate at least one prevention to a target"
            );
        } else {
            lk::use_ability(&mut e, p0, serra_s_hymn(), 1, &[], &[p0]);
        }
        lk::float(&mut e, p0, &[(ManaColor::Red, 2)]);
        lk::cast_at(&mut e, p0, lightning_bolt(), &[], &[p1]);
        assert_eq!(lk::life(&e, p1), 17, "other targets are not protected");
        lk::cast_at(&mut e, p0, lightning_bolt(), &[], &[p0]);
        assert_eq!(lk::life(&e, p0), 17 + i32::from(n));
    }
}
