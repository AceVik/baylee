//! `cards/instants/mv_3/mana_short.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mana Short: "Tap all lands target player controls. That player loses all
/// unspent mana." Self-targeted: some of p0's lands pay for the spell
/// itself, two more are tapped afterward for mana that is never spent, and
/// one more is left standing untouched — proving the tap reaches every
/// land, not only the ones a cost already used.
#[test]
fn mana_short_taps_all_lands_and_empties_the_pool() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[island(), island(), forest(), swamp(), swamp(), mountain()],
        )
        .hand(0, &[mana_short()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let payers: Vec<ObjectId> = all_on_battlefield(&engine, p0, island())
        .into_iter()
        .chain(all_on_battlefield(&engine, p0, forest()))
        .collect();
    tap_mana_where(&mut engine, p0, |id| payers.contains(&id));
    cast_with_floating(&mut engine, p0, mana_short());
    engine
        .apply(p0, PlayerAction::ChoosePlayer(p0))
        .expect("p0 targets itself");

    let swamps = all_on_battlefield(&engine, p0, swamp());
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two black floating, unspent"
    );
    let a_mountain = on_battlefield(&engine, p0, mountain()).expect("seated");
    assert!(!is_tapped(&engine, a_mountain), "untouched so far");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine
        .apply(PlayerId::new(1), PlayerAction::PassPriority)
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "\"loses all unspent mana\""
    );
    assert!(
        is_tapped(&engine, a_mountain),
        "\"tap all lands target player controls\" — even one never used for mana"
    );
}
