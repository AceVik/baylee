//! `cards/lands/planets/uthros_titanic_godcore.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The land enters tapped. Without charge counters or a creature to station,
/// only the ordinary blue mana ability is offered after it untaps.
#[test]
fn uthros_titanic_godcore_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[uthros_titanic_godcore()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, uthros_titanic_godcore());
    assert!(entered_tapped(&engine, land), "enters tapped");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.abilities.iter().filter(|(id, _)| *id == land).count(),
        1,
        "only ability 0 is offered because charge counter threshold is not met"
    );

    activate(&mut engine, p0, uthros_titanic_godcore(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(is_tapped(&engine, land));
}

#[test]
fn uthros_stations_to_twelve_and_counts_only_your_artifacts() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(
        952,
        &[
            uthros_titanic_godcore(),
            llanowar_elves(),
            sol_ring(),
            sol_ring(),
        ],
        &[sol_ring()],
        &[],
        &[],
    );
    let land = kit::obj(&engine, kit::P0, uthros_titanic_godcore());
    let elf = kit::obj(&engine, kit::P0, llanowar_elves());
    crate::replacement::put_counters(
        engine.dev_state_mut(kit::P0).unwrap(),
        land,
        CounterKind::Charge,
        11,
    );
    kit::float(&mut engine, kit::P0, &[(ManaColor::Blue, 1)]);
    assert!(!kit::offered(&engine, uthros_titanic_godcore(), kit::P0, 1));
    activate(&mut engine, kit::P0, uthros_titanic_godcore(), 2);
    engine
        .apply(kit::P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    kit::settle(&mut engine);
    assert_eq!(counters_on(&engine, land, CounterKind::Charge), 12);
    activate(&mut engine, kit::P0, uthros_titanic_godcore(), 1);
    assert_eq!(kit::pool_of(&engine, kit::P0, ManaColor::Blue), 2);
}
