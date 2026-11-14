use super::*;
use baylee_core::generated::index::{CONTROL_MAGIC, DARKSTEEL_CITADEL, GRIZZLY_BEARS, HILL_GIANT};

fn upheaval() -> CardIndex {
    card_index("7cafc972-a6f5-4cac-a3d3-8a3ae36ffb1e")
}

#[test]
fn upheaval_returns_all_permanents_of_both_players_to_owners_hands() {
    let up = upheaval();
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
                GRIZZLY_BEARS,
            ],
        )
        .battlefield(
            1,
            &[
                forest(),
                swamp(),
                mountain(),
                HILL_GIANT,
                sol_ring(),
                jace_the_mind_sculptor(),
            ],
        )
        .hand(0, &[up])
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

    cast_from_hand(&mut engine, p0, up);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .is_empty(),
        "no permanent remains on the battlefield"
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 7);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p1)).len(), 6);
    assert!(in_hand(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, p0, island()).is_some());
    assert!(in_hand(&engine, p1, HILL_GIANT).is_some());
    assert!(in_hand(&engine, p1, sol_ring()).is_some());
    assert!(in_hand(&engine, p1, jace_the_mind_sculptor()).is_some());
    assert!(in_hand(&engine, p1, forest()).is_some());
    assert!(in_graveyard(&engine, p0, up).is_some());
}

#[test]
fn upheaval_returns_stolen_creature_and_aura_to_owners_plus_indestructible() {
    let up = upheaval();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[island(), island(), plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[HILL_GIANT, DARKSTEEL_CITADEL])
        .hand(0, &[CONTROL_MAGIC, up])
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
    cast_from_hand(&mut engine, p0, up);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .is_empty()
    );
    assert!(in_hand(&engine, p0, HILL_GIANT).is_none());
    assert_eq!(in_hand(&engine, p1, HILL_GIANT), Some(giant));
    assert!(in_hand(&engine, p0, CONTROL_MAGIC).is_some());
    assert!(in_hand(&engine, p1, DARKSTEEL_CITADEL).is_some());
}
