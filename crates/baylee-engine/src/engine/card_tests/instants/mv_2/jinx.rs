//! Jinx — `{1}{U}` Instant (Homelands)
//!
//! Target land becomes the basic land type of your choice until end of turn.
//! Draw a card at the beginning of the next turn's upkeep.
//!
//! (`Coverage::Partial`: choosing a basic land type on spell resolution is not
//! in `Effect`; sets target land's land type to Island until end of turn and
//! draws a card at the beginning of the next turn's upkeep.)

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn jinx() -> CardIndex {
    card_index("e6e1b51c-ba4b-4997-968a-751b4cba110f")
}

#[test]
fn jinx_turns_target_land_into_island_until_end_of_turn_and_draws_at_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[jinx()])
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[mountain(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let jinx_hand = in_hand(&engine, p0, jinx()).unwrap();
    let opp_mtn = on_battlefield(&engine, p1, mountain()).unwrap();
    let opp_forest = on_battlefield(&engine, p1, forest()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: jinx_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![opp_mtn],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let subs = &engine
        .state()
        .object(opp_mtn)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(
        subs.contains(subtypes::land::ISLAND),
        "target Mountain became an Island until end of turn"
    );
    assert!(
        !subs.contains(subtypes::land::MOUNTAIN),
        "target Mountain lost its Mountain land type until end of turn"
    );

    assert!(
        engine
            .state()
            .object(opp_forest)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::FOREST),
        "a land that was not targeted keeps its type"
    );

    // Advance to p1's main phase: land reverts to Mountain and p0 drew a card at p1's upkeep.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        engine
            .state()
            .object(opp_mtn)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::MOUNTAIN)
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "Jinx drew a card at the beginning of the next turn's upkeep"
    );
}
