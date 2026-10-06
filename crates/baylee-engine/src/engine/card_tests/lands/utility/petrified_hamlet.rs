//! `cards/lands/utility/petrified_hamlet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Petrified Hamlet prints `When this land enters, choose a land card name.`, `Activated abilities of sources with the chosen name can't be activated unless they're mana abilities.`, `Lands with the chosen name have "{{T}}: Add {{C}}."`, and `{{T}}: Add {{C}}.`
///
/// Under `Coverage::Partial`, the name-choosing ETB and static ability lock are omitted because naming card choices and name-based activation locks are unsupported.
/// Playing Petrified Hamlet from hand enters the battlefield untapped and leaves the player at quiet priority without prompting for a card name.
/// Activating ability 0 produces one colorless mana in `pool.available(ManaColor::Colorless)` and leaves the land tapped.
#[test]
fn petrified_hamlet_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[petrified_hamlet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hamlet = play_land(&mut engine, p0, petrified_hamlet());
    assert!(
        !entered_tapped(&engine, hamlet),
        "petrified hamlet enters untapped"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no ETB name choice prompt is asked under `Coverage::Partial`"
    );

    activate(&mut engine, p0, petrified_hamlet(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, hamlet));
}
