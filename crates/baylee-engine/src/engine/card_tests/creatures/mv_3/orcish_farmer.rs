//! `cards/creatures/mv_3/orcish_farmer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn orcish_farmer() -> CardIndex {
    card_index("c3039d19-8c98-4953-8943-9922b6ab45ef")
}

/// Orcish Farmer (c3039d19-8c98-4953-8943-9922b6ab45ef) — {1}{R}{R} 2/2 Creature — Orc:
/// "{T}: Target land becomes a Swamp until its controller's next untap step."
/// (Coverage::Partial — lasts until end of turn instead of until its controller's next untap step.)
#[test]
fn orcish_farmer_turns_target_land_into_a_swamp_that_taps_for_black_mana() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[orcish_farmer(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let fst = on_battlefield(&engine, P0, forest()).unwrap();
    activate(&mut engine, P0, orcish_farmer(), 0);
    aim_at(&mut engine, P0, fst);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(fst)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::land::SWAMP),
        "target Forest becomes a Swamp"
    );
    tap_mana_where(&mut engine, P0, |id| id == fst);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "the land now taps for {{B}}"
    );
}
