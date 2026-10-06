//! `cards/lands/utility/timberline_ridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Timberline Ridge` prints `This land doesn't untap during your untap step if it has a depletion counter on it.`, `At the beginning of your upkeep, remove a depletion counter from this land.`, and `{{T}}: Add {{R}} or {{G}}. Put a depletion counter on this land.`
///
/// Under `Coverage::Partial`, the untap lock and upkeep removal are omitted because static abilities carry no condition and effects do not remove counters.
/// Playing `Timberline Ridge` from hand enters untapped.
/// Activating ability 0 prompts with `Pending::ChooseColor` between `ManaColor::Red` and `ManaColor::Green`. Choosing red adds one red mana and places a `counters::DEPLETION` counter on `Timberline Ridge`.
/// Advancing to the next own main phase verifies that the unmodeled upkeep clause does not remove the counter and the unmodeled untap lock leaves the land untapped.
#[test]
fn timberline_ridge_adds_chosen_mana_and_depletion_counter() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[timberline_ridge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ridge = play_land(&mut engine, p0, timberline_ridge());
    assert!(
        !entered_tapped(&engine, ridge),
        "timberline ridge prints no enters-tapped clause"
    );

    activate(&mut engine, p0, timberline_ridge(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![ManaColor::Red, ManaColor::Green]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert_eq!(counters_on(&engine, ridge, counters::DEPLETION), 1);
    assert!(is_tapped(&engine, ridge));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        counters_on(&engine, ridge, counters::DEPLETION),
        1,
        "upkeep removal is not built under `Coverage::Partial`"
    );
    assert!(
        !is_tapped(&engine, ridge),
        "untap lock is not built under `Coverage::Partial`"
    );
}
