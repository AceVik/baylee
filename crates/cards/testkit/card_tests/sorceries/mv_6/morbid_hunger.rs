use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn morbid_hunger() -> CardIndex {
    card_index("6e7e9685-ebe9-4db2-b52c-f6ce95e37e56")
}

#[test]
fn morbid_hunger_deals_three_to_creature_and_gains_three_life() {
    let mh = morbid_hunger();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[swamp(), swamp(), plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[mh])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, p0, mh);
    let menu = aim_at(&mut engine, p0, giant);
    assert!(menu.contains(&giant));
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p1, HILL_GIANT).is_none());
    assert_eq!(in_graveyard(&engine, p1, HILL_GIANT), Some(giant));
    assert_eq!(super::life_of(&engine, p0), 23);
    assert_eq!(super::life_of(&engine, p1), 20);
    assert!(in_graveyard(&engine, p0, mh).is_some());
    assert_eq!(exiled(&engine, p0, mh), 0);
}

#[test]
fn morbid_hunger_flashback_hits_player_gains_three_and_exiles() {
    let mh = morbid_hunger();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[mh])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let gy_id = hand_to_graveyard(&mut engine, p0, mh);
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
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(super::life_of(&engine, p1), 17);
    assert_eq!(super::life_of(&engine, p0), 23);
    assert!(in_graveyard(&engine, p0, mh).is_none());
    assert_eq!(exiled(&engine, p0, mh), 1);
}

#[test]
fn morbid_hunger_fizzles_gains_no_life_and_rejects_short_flashback() {
    let mh = morbid_hunger();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[mh, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, mh);
    aim_at(&mut engine, p0, bears);
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
    assert_eq!(
        super::life_of(&engine, p0),
        20,
        "CR 608.2b: fizzled Morbid Hunger must not gain 3 life"
    );
    let gy_mh = in_graveyard(&engine, p0, mh).unwrap();

    // Next turn: 8 mana is 1 short of flashback {7}{B}{B}.
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: gy_mh })
            .is_err(),
        "flashback {{7}}{{B}}{{B}} must fail with 8 mana"
    );
}
