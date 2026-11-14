use super::*;
use baylee_core::generated::index::{DARKSTEEL_CITADEL, GRIZZLY_BEARS};

fn earth_rift() -> CardIndex {
    card_index("888a5071-24b1-48ce-a97b-39f108976c6d")
}

#[test]
fn earth_rift_targets_and_destroys_only_lands() {
    let er = earth_rift();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[mountain(), plains(), plains(), plains()])
        .battlefield(1, &[forest(), DARKSTEEL_CITADEL, GRIZZLY_BEARS, sol_ring()])
        .hand(0, &[er])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let p1_forest = on_battlefield(&engine, p1, forest()).unwrap();
    let citadel = on_battlefield(&engine, p1, DARKSTEEL_CITADEL).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();

    cast_from_hand(&mut engine, p0, er);
    match engine.pending().clone() {
        Pending::ChooseTargets { options, .. } => {
            assert!(options.contains(&p1_forest) && options.contains(&citadel));
            assert!(!options.contains(&bears) && !options.contains(&ring));
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
        "must not target nonland creature"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![p1_forest],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p1, forest()).is_none());
    assert_eq!(in_graveyard(&engine, p1, forest()), Some(p1_forest));
    assert!(in_graveyard(&engine, p0, er).is_some());
    assert_eq!(exiled(&engine, p0, er), 0);
}

#[test]
fn earth_rift_flashback_destroys_land_and_exiles_self() {
    let er = earth_rift();
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
            ],
        )
        .battlefield(1, &[swamp()])
        .hand(0, &[er])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let gy_id = hand_to_graveyard(&mut engine, p0, er);
    let p1_swamp = on_battlefield(&engine, p1, swamp()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: gy_id })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let mode = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    }
    aim_at(&mut engine, p0, p1_swamp);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(in_graveyard(&engine, p1, swamp()), Some(p1_swamp));
    assert!(in_graveyard(&engine, p0, er).is_none());
    assert_eq!(exiled(&engine, p0, er), 1);
}

#[test]
fn earth_rift_boundaries_indestructible_land_survives_and_short_flashback() {
    let er = earth_rift();
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
            ],
        )
        .battlefield(1, &[DARKSTEEL_CITADEL])
        .hand(0, &[er])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let citadel = on_battlefield(&engine, p1, DARKSTEEL_CITADEL).unwrap();
    cast_from_hand(&mut engine, p0, er);
    aim_at(&mut engine, p0, citadel);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        on_battlefield(&engine, p1, DARKSTEEL_CITADEL),
        Some(citadel),
        "indestructible land must survive destroy"
    );
    let gy_er = in_graveyard(&engine, p0, er).unwrap();

    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: gy_er })
            .is_err(),
        "flashback {{5}}{{R}}{{R}} must reject 6 mana"
    );
}
