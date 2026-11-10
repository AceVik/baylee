//! `cards/lands/manlands/creeping_tar_pit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Creeping Tar Pit is `Coverage::Implemented`.  It enters tapped and has:
/// index 0 — `{T}: Add {U} or {B}` (colour-choice mana ability); and
/// index 1 — `{1}{U}{B}: Until end of turn, this land becomes a 3/2 blue and
/// black Elemental creature with "can't be blocked".  It's still a land.`
///
/// Three things are proved:
/// 1. The land enters tapped.
/// 2. After animation the permanent is both `CREATURE` and `LAND`, `3/2`, and
///    carries `KeywordSet::UNBLOCKABLE`.
/// 3. The colour-choice mana ability (index 0) is distinct from the CR 305.6
///    shortcut and correctly asks for a colour choice.
///
/// Animation costs `{1}{U}{B}` with no `{T}`, so the land need not be tapped
/// to pay it.  With Island + Swamp for `{U}{B}` and the Tar Pit's own mana
/// ability for the remaining generic `{1}`, the three mana are covered even
/// with no extra lands.
#[test]
fn creeping_tar_pit_enters_tapped_then_animates_into_a_3_2_unblockable_elemental() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // The Tar Pit comes from hand on turn 1 (enters tapped).
    // Animation and mana are exercised on turn 2 after it untaps.
    let mut engine = Duel::new(73, island())
        .battlefield(0, &[island(), swamp()])
        .hand(0, &[creeping_tar_pit()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, creeping_tar_pit());
    assert!(
        entered_tapped(&engine, land),
        "Creeping Tar Pit enters tapped"
    );

    // Pass to opponent's turn, then back to p0's next main phase.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    reach_their_main_phase(&mut engine, p0);

    // The land untapped in p0's untap step.
    assert!(
        !is_tapped(&engine, land),
        "the untap step gave the land back"
    );
    assert!(
        !engine
            .state()
            .object(land)
            .expect("still on the battlefield")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "it is not a creature before the activation"
    );

    // Animation costs {1}{U}{B} (no {T}).  Mana plan:
    //   • Island (mana_abilities shortcut) → {U}
    //   • Swamp (mana_abilities shortcut) → {B}
    //   • Tar Pit ability 0 ({T}: Add {U} or {B}) → asks colour, tap the Pit for {U}
    // Together {U}{U}{B} covers {1}{U}{B} (the {1} is satisfied by {U}).
    // Tap the basics and *name* the Pit, because its own `{T}: Add {U} or
    // {B}` is a mana ability whose whole cost is its tap symbol -- which is
    // exactly what `tap_all_mana` presses, leaving the activation below to
    // be refused for a land that is already tapped.
    tap_all_mana_but(&mut engine, p0, Some(creeping_tar_pit()));

    // Now activate the Tar Pit's mana ability (index 0) for one more colour.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the printed mana ability activates");
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "the colour-choice mana ability asks which colour: {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "both {{U}} and {{B}} are offered: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    // Pool: {U}{U}{B} — covers {1}{U}{B}.

    // The Tar Pit is now tapped (from its mana ability).  The animation ability
    // costs {1}{U}{B} — no {T} — so the tapped land can still use it.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("{U}{U}{B} pays {1}{U}{B}");
    pass_until(&mut engine, |e| {
        e.state()
            .object(land)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
    });

    let obj = engine
        .state()
        .object(land)
        .expect("the Tar Pit is still an object");
    let types = obj.characteristics().types;
    assert!(types.contains(TypeSet::CREATURE), "it became a creature");
    assert!(types.contains(TypeSet::LAND), "it's still a land");
    assert_eq!(pt(&engine, land), (3, 2), "3/2 as printed");
    assert!(
        obj.characteristics()
            .keywords
            .contains(KeywordSet::UNBLOCKABLE),
        "can't be blocked"
    );
}
