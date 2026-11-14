use super::*;
use baylee_core::generated::index::{CONTROL_MAGIC, GRIZZLY_BEARS, HILL_GIANT};

fn dematerialize() -> CardIndex {
    card_index("d2fbad7d-d05a-4321-9aff-a93bd102a3f4")
}

#[test]
fn dematerialize_bounces_any_permanent_to_owners_hand() {
    let dm = dematerialize();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS, sol_ring(), swamp()])
        .hand(0, &[dm, dm, dm])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let p1_swamp = on_battlefield(&engine, p1, swamp()).unwrap();

    cast_from_hand(&mut engine, p0, dm);
    match engine.pending().clone() {
        Pending::ChooseTargets { options, .. } => {
            assert!(
                options.contains(&bears) && options.contains(&ring) && options.contains(&p1_swamp)
            );
        }
        other => panic!("expected ChooseTargets, got {other:?}"),
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![p1_swamp],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p1, swamp()).is_none());
    assert_eq!(in_hand(&engine, p1, swamp()), Some(p1_swamp));
    assert!(in_graveyard(&engine, p0, dm).is_some());
    assert_eq!(exiled(&engine, p0, dm), 0);
}

#[test]
fn dematerialize_returns_stolen_permanent_to_owners_hand() {
    let dm = dematerialize();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[island(), island(), plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[CONTROL_MAGIC, dm])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, p0, CONTROL_MAGIC);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(on_battlefield(&engine, p0, HILL_GIANT), Some(giant));

    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    cast_from_hand(&mut engine, p0, dm);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_hand(&engine, p0, HILL_GIANT).is_none());
    assert_eq!(in_hand(&engine, p1, HILL_GIANT), Some(giant));
    assert!(in_graveyard(&engine, p0, CONTROL_MAGIC).is_some());
}

#[test]
fn dematerialize_flashback_bounces_exiles_and_rejects_short_mana() {
    let dm = dematerialize();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[island(), island(), plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[dm])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let gy_id = hand_to_graveyard(&mut engine, p0, dm);
    // 6 mana is 1 short of flashback {5}{U}{U}.
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: gy_id })
            .is_err(),
        "flashback {{5}}{{U}}{{U}} must fail with only 6 mana"
    );

    // Give P0 a 7th land via fresh turn + extra plains on board in new duel,
    // or seed 7th land on a dedicated flashback pass cleanly:
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                island(),
                island(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[dm])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let gy_id = hand_to_graveyard(&mut engine, p0, dm);
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: gy_id })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let mode = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    }
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(in_hand(&engine, p1, GRIZZLY_BEARS), Some(bears));
    assert!(in_graveyard(&engine, p0, dm).is_none());
    assert_eq!(exiled(&engine, p0, dm), 1);
}
