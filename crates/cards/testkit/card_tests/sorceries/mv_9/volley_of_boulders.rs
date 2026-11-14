use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn volley_of_boulders() -> CardIndex {
    card_index("47860e92-e86a-49f2-80e3-fd144de08b4d")
}

#[test]
fn volley_of_boulders_divides_six_damage_across_multiple_targets() {
    let vb = volley_of_boulders();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[mountain(); 9])
        .battlefield(1, &[GRIZZLY_BEARS, HILL_GIANT])
        .hand(0, &[vb])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, vb);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant],
                players: vec![p1],
            },
        )
        .unwrap();

    // CR 601.2d: each chosen target must receive at least 1 damage.
    assert!(matches!(
        engine.pending(),
        Pending::ChooseNumber { min: 1, max: 4, .. }
    ));
    assert!(
        engine.apply(p0, PlayerAction::ChooseNumber(0)).is_err(),
        "CR 601.2d: share of 0 damage is illegal"
    );
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseNumber { min: 1, max: 3, .. }
    ));
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(in_graveyard(&engine, p1, GRIZZLY_BEARS), Some(bears));
    assert_eq!(in_graveyard(&engine, p1, HILL_GIANT), Some(giant));
    assert_eq!(life_of(&engine, p1), 19);
    assert!(in_graveyard(&engine, p0, vb).is_some());
    assert_eq!(exiled(&engine, p0, vb), 0);
}

#[test]
fn volley_of_boulders_flashback_divides_six_for_six_red_and_exiles() {
    let vb = volley_of_boulders();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[mountain(); 6])
        .battlefield(1, &[GRIZZLY_BEARS, jace_the_mind_sculptor()])
        .hand(0, &[vb])
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

    let gy_id = hand_to_graveyard(&mut engine, p0, vb);
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: gy_id })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let mode = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, jace],
                players: vec![p1],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p1, jace_the_mind_sculptor()).is_some());
    assert_eq!(life_of(&engine, p1), 19);
    assert!(in_graveyard(&engine, p0, vb).is_none());
    assert_eq!(exiled(&engine, p0, vb), 1);
}

#[test]
fn volley_of_boulders_boundaries_zero_targets_max_six_and_short_flashback() {
    let vb = volley_of_boulders();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(
            1,
            &[
                GRIZZLY_BEARS,
                GRIZZLY_BEARS,
                GRIZZLY_BEARS,
                GRIZZLY_BEARS,
                GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[vb])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let five_bears = all_on_battlefield(&engine, p1, GRIZZLY_BEARS);
    assert_eq!(five_bears.len(), 5);

    cast_from_hand(&mut engine, p0, vb);
    match engine.pending().clone() {
        Pending::ChooseTargets { min, max, .. } => {
            assert_eq!((min, max), (0, 6));
        }
        other => panic!("expected ChooseTargets, got {other:?}"),
    }
    // 7 targets exceeds max of 6.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: five_bears,
                    players: vec![p0, p1],
                },
            )
            .is_err(),
        "CR 601.2d: cannot name more than 6 targets for 6 divided damage"
    );

    // 0 targets is valid ("any number of targets"): no division prompt, no damage.
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(life_of(&engine, p0), 20);
    assert_eq!(life_of(&engine, p1), 20);
    let gy_vb = in_graveyard(&engine, p0, vb).unwrap();

    // Next turn: 5 red + 4 white (9 total, only 5 red) cannot pay {R}{R}{R}{R}{R}{R}.
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: gy_vb })
            .is_err(),
        "flashback requires six red mana specifically"
    );
}
