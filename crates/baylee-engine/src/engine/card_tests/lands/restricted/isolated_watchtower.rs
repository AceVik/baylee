//! `cards/lands/restricted/isolated_watchtower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Isolated Watchtower prints `{{T}}: Add {{C}}.` and `{2}, {{T}}: Scry 1, then you
/// may reveal the top card of your library. If a basic land card is revealed this way,
/// put it onto the battlefield tapped. Activate only if an opponent controls at least
/// two more lands than you.`
///
/// Under `Coverage::Partial`, the activated scry and land-reveal ability is omitted
/// because no condition compares land counts across players. This test sets up an
/// opponent with three lands against one, verifies that only the mana ability is
/// offered, and taps it for one colorless mana.
#[test]
fn isolated_watchtower_offers_only_mana_ability_when_behind_on_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[isolated_watchtower()])
        .battlefield(1, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tower =
        on_battlefield(&engine, p0, isolated_watchtower()).expect("watchtower on battlefield");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal
            .abilities
            .iter()
            .filter(|(id, _)| *id == tower)
            .count(),
        1,
        "only ability 0 is offered under Coverage::Partial"
    );

    activate(&mut engine, p0, isolated_watchtower(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, tower));
}
