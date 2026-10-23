//! `cards/enchantments/auras/mv_2/festering_wound.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn festering_wound() -> CardIndex {
    card_index("32638d03-c101-42f0-8a4b-8c6290257a28")
}

#[test]
fn festering_wound_accumulates_infection_counters_and_causes_life_loss() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[festering_wound()])
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, festering_wound());
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    // The first opponent upkeep has zero infection counters.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 20);
    // The Aura controller may decline to grow the wound.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 20);
    for expected in [19, 17] {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::YesNo { .. })
        });
        let Pending::YesNo { player, .. } = engine.pending().clone() else {
            unreachable!()
        };
        assert_eq!(player, p0);
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().players[0].life,
            20,
            "Aura controller's upkeep is not the damage trigger"
        );
        reach_their_main_phase(&mut engine, p1);
        assert_eq!(engine.state().players[1].life, expected);
    }
}
