//! `cards/instants/mv_3/mangara_s_blessing.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "5d9431de-7cf0-43d4-9b86-e0ef4a7756f9"
fn mangara_s_blessing() -> CardIndex {
    card_index("5d9431de-7cf0-43d4-9b86-e0ef4a7756f9")
}

#[test]
fn mangara_s_blessing_cast_gains_5_life_and_discarded_by_opponent_gains_2_life_and_returns_at_end_step()
 {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[mangara_s_blessing(), mangara_s_blessing()])
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(1, &[index::THOUGHTSEIZE])
        .battlefield(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 1. Cast Mangara's Blessing normally -> gains 5 life.
    let mb1 = in_hand(&engine, p0, mangara_s_blessing()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: mb1 })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 25);

    // 2. Advance to P1's main phase. P1 casts Thoughtseize targeting P0, discarding the second Mangara's Blessing.
    reach_their_main_phase(&mut engine, p1);
    let ts = in_hand(&engine, p1, index::THOUGHTSEIZE).unwrap();
    let mb2 = in_hand(&engine, p0, mangara_s_blessing()).unwrap();

    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: ts })
        .unwrap();
    engine.apply(p1, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![mb2] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        27,
        "Discarded Mangara's Blessing trigger gained 2 life"
    );
}

/// "A spell or ability an opponent controls causes you to discard THIS card":
/// an opponent's Thoughtseize that takes a different card gains no life, and
/// the Blessing stays in hand.
#[test]
fn mangara_s_blessing_gains_no_life_when_an_opponent_discards_another_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[mangara_s_blessing(), giant_growth()])
        .hand(1, &[index::THOUGHTSEIZE])
        .battlefield(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    reach_their_main_phase(&mut engine, p1);
    let ts = in_hand(&engine, p1, index::THOUGHTSEIZE).unwrap();
    let growth = in_hand(&engine, p0, giant_growth()).unwrap();

    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: ts })
        .unwrap();
    engine.apply(p1, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![growth],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        20,
        "the discarded card was Giant Growth, not the Blessing"
    );
    assert!(in_hand(&engine, p0, mangara_s_blessing()).is_some());
    assert!(in_graveyard(&engine, p0, giant_growth()).is_some());
}
