use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn scorching_missile() -> CardIndex {
    card_index("02e2dbde-2373-4f29-bd9d-787f1805b061")
}

#[test]
fn scorching_missile_hits_player_or_planeswalker_and_excludes_creatures() {
    let sm = scorching_missile();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[GRIZZLY_BEARS, jace_the_mind_sculptor()])
        .hand(0, &[sm, sm])
        .start();
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let jace = on_battlefield(&engine, p1, jace_the_mind_sculptor()).unwrap();
    engine
        .dev_state_mut(p1)
        .unwrap()
        .object_mut(jace)
        .unwrap()
        .counters
        .set(CounterKind::Loyalty, 5);
    engine.refresh_offer();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, sm);
    match engine.pending().clone() {
        Pending::ChooseTargets {
            options,
            player_options,
            ..
        } => {
            assert!(options.contains(&jace));
            assert!(!options.contains(&bears), "must not target creature");
            assert!(player_options.contains(&p0) && player_options.contains(&p1));
        }
        other => panic!("expected ChooseTargets, got {other:?}"),
    }
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![],
                },
            )
            .is_err(),
        "creature is illegal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(life_of(&engine, p1), 16);

    // Second cast hits 5-loyalty planeswalker -> drops to 1 loyalty.
    cast_from_hand(&mut engine, p0, sm);
    aim_at(&mut engine, p0, jace);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(counters_on(&engine, jace, CounterKind::Loyalty), 1);
    assert_eq!(
        on_battlefield(&engine, p1, jace_the_mind_sculptor()),
        Some(jace)
    );
    assert_eq!(exiled(&engine, p0, sm), 0);
}

#[test]
fn scorching_missile_flashback_kills_planeswalker_and_exiles() {
    let sm = scorching_missile();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[jace_the_mind_sculptor()])
        .hand(0, &[sm])
        .start();
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let jace = on_battlefield(&engine, p1, jace_the_mind_sculptor()).unwrap();
    engine
        .dev_state_mut(p1)
        .unwrap()
        .object_mut(jace)
        .unwrap()
        .counters
        .set(CounterKind::Loyalty, 3);
    engine.refresh_offer();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gy_id = hand_to_graveyard(&mut engine, p0, sm);
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: gy_id })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let mode = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    }
    aim_at(&mut engine, p0, jace);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p1, jace_the_mind_sculptor()).is_none());
    assert_eq!(
        in_graveyard(&engine, p1, jace_the_mind_sculptor()),
        Some(jace)
    );
    assert!(in_graveyard(&engine, p0, sm).is_none());
    assert_eq!(exiled(&engine, p0, sm), 1);
}

#[test]
fn scorching_missile_boundaries_self_target_and_short_flashback() {
    let sm = scorching_missile();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[sm, sm])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    reach_main_phase(&mut engine, p0);

    let gy_id = hand_to_graveyard(&mut engine, p0, sm);
    // 9 mana is 1 short of flashback {9}{R}.
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: gy_id })
            .is_err(),
        "flashback {{9}}{{R}} must fail with 9 mana"
    );

    // Can target self (uses 4 of the 9 floating mana).
    cast_with_floating(&mut engine, p0, sm);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(life_of(&engine, p0), 16);
}
