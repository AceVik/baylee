//! `cards/creatures/mv_3/orcish_farmer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn orcish_farmer() -> CardIndex {
    card_index("c3039d19-8c98-4953-8943-9922b6ab45ef")
}

fn is_swamp(engine: &Engine<RegistryLookup>, land: ObjectId) -> bool {
    engine
        .state()
        .object(land)
        .unwrap()
        .characteristics()
        .subtypes
        .contains(baylee_core::generated::subtypes::land::SWAMP)
}

fn is_forest(engine: &Engine<RegistryLookup>, land: ObjectId) -> bool {
    engine
        .state()
        .object(land)
        .unwrap()
        .characteristics()
        .subtypes
        .contains(baylee_core::generated::subtypes::land::FOREST)
}

/// Orcish Farmer (c3039d19-8c98-4953-8943-9922b6ab45ef) — {1}{R}{R} 2/2 Creature — Orc:
/// "{T}: Target land becomes a Swamp until its controller's next untap step."
#[test]
fn orcish_farmer_turns_your_land_into_a_swamp_until_your_next_untap_step() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[orcish_farmer(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let fst = on_battlefield(&engine, P0, forest()).unwrap();
    assert!(is_forest(&engine, fst) && !is_swamp(&engine, fst));
    activate(&mut engine, P0, orcish_farmer(), 0);
    aim_at(&mut engine, P0, fst);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_swamp(&engine, fst), "target Forest becomes a Swamp");
    assert!(
        !is_forest(&engine, fst),
        "setting a basic land type replaces the old one (CR 305.7)"
    );
    tap_mana_where(&mut engine, P0, |id| id == fst);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "the land now taps for {{B}}"
    );

    // Past the end of the turn and through the opponent's untap step: the land is P0's, and
    // only P0's untap step ends the effect.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        is_swamp(&engine, fst),
        "still a Swamp in the opponent's turn: its controller has not had an untap step yet"
    );

    // P0's next untap step (turn 3) ends it.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        !is_swamp(&engine, fst) && is_forest(&engine, fst),
        "the land is a Forest again after its controller's next untap step"
    );
}

#[test]
fn orcish_farmer_can_target_an_opponents_land_but_not_a_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[orcish_farmer()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let farmer = on_battlefield(&engine, P0, orcish_farmer()).unwrap();
    let their_forest = on_battlefield(&engine, P1, forest()).unwrap();
    activate(&mut engine, P0, orcish_farmer(), 0);
    let options = aim_at(&mut engine, P0, their_forest);
    assert!(
        !options.contains(&farmer),
        "only a land is a legal target, not the creature"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_swamp(&engine, their_forest),
        "target land can be an opponent's"
    );

    // Their untap step (turn 2) is the end of it.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        is_forest(&engine, their_forest) && !is_swamp(&engine, their_forest),
        "the opponent's land is a Forest again after its controller's next untap step"
    );
}
