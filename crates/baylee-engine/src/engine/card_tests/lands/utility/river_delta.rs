//! `cards/lands/utility/river_delta.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// River Delta prints `This land doesn't untap during your untap step if it has a depletion counter on it.`, `At the beginning of your upkeep, remove a depletion counter from this land.`, and `{{T}}: Add {{U}} or {{B}}. Put a depletion counter on this land.`
///
/// Under `Coverage::Partial`, the untap lock and upkeep counter removal are omitted because static abilities carry no condition and effects do not remove counters.
/// Playing River Delta from hand enters untapped.
/// Activating ability 0 prompts with `Pending::ChooseColor` between `ManaColor::Blue` and `ManaColor::Black`. Choosing blue adds one blue mana and places a `counters::DEPLETION` counter on River Delta.
/// Advancing to the next own main phase verifies that the unmodeled upkeep clause does not remove the counter and the unmodeled untap lock leaves the land untapped.
#[test]
fn river_delta_adds_chosen_mana_and_depletion_counter() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[river_delta()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let delta = play_land(&mut engine, p0, river_delta());
    assert!(
        !entered_tapped(&engine, delta),
        "river delta enters untapped"
    );

    activate(&mut engine, p0, river_delta(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![ManaColor::Blue, ManaColor::Black]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert_eq!(counters_on(&engine, delta, counters::DEPLETION), 1);
    assert!(is_tapped(&engine, delta));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        counters_on(&engine, delta, counters::DEPLETION),
        1,
        "upkeep removal is not built under `Coverage::Partial`"
    );
    assert!(
        !is_tapped(&engine, delta),
        "untap lock is not built under `Coverage::Partial`"
    );
}
