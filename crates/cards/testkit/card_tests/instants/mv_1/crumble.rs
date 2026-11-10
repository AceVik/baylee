//! `cards/instants/mv_1/crumble.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crumble — {G} — "Destroy target artifact. It can't be regenerated. That
/// artifact's controller gains life equal to its mana value."
///
/// A shield bought from the Living Wall's own `{1}` ability is standing when
/// the spell resolves, and "can't be regenerated" is why it is not applied
/// (CR 701.19c): the Wall goes to its owner's graveyard anyway. The life
/// follows *that artifact's* controller (CR 608.2h — the destroy ahead of it
/// in the same sentence has already moved the Wall), so p1 gains its mana
/// value, four, and p0 gains nothing. The Pendant beside the spell is a
/// legal target that was not named and never moves.
#[test]
fn crumble_destroys_through_a_shield_and_its_controller_gains_its_mana_value() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), darksteel_pendant()])
        .battlefield(1, &[living_wall(), forest()])
        .hand(0, &[crumble()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p1, living_wall()).expect("their Wall is out");
    let pendant = on_battlefield(&engine, p0, darksteel_pendant()).expect("my Pendant is out");

    // p1 shields the Wall before the spell, which is the board a "can't be
    // regenerated" clause is printed for.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    engine
        .apply(
            p1,
            PlayerAction::ActivateManaAbility {
                source: their_forest,
            },
        )
        .unwrap();
    activate(&mut engine, p1, living_wall(), 0);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine
            .state()
            .object(wall)
            .expect("the Wall is still there")
            .regeneration_shields,
        1,
        "one shield, standing over the Wall"
    );

    cast_from_hand(&mut engine, p0, crumble());
    let menu = aim_at(&mut engine, p0, wall);
    assert!(
        menu.contains(&wall) && menu.contains(&pendant),
        "\"target artifact\" is any artifact, on either side of the table: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, living_wall()).is_none(),
        "\"It can't be regenerated\": the shield did not save the Wall"
    );
    assert!(in_graveyard(&engine, p1, living_wall()).is_some());
    assert_eq!(
        engine.state().players[1].life,
        24,
        "\"That artifact's controller gains life equal to its mana value\": four"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the caster is not that controller"
    );
    assert!(
        on_battlefield(&engine, p0, darksteel_pendant()).is_some(),
        "the artifact the spell did not name never moved"
    );
}

/// The same sentence against Scryfall's 2004 ruling: "If the target artifact
/// becomes illegal before resolution, the player does not gain any life."
/// The second Crumble resolves first and destroys the Wall; the first then
/// has no legal target (CR 608.2b), so it neither destroys nor gains — four
/// life once, not eight.
#[test]
fn crumble_gains_nothing_when_its_target_is_gone_by_resolution() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[living_wall(), forest(), forest()])
        .hand(0, &[crumble(), crumble()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p0, living_wall()).expect("the Wall is out");
    cast_from_hand(&mut engine, p0, crumble());
    aim_at(&mut engine, p0, wall);
    cast_with_floating(&mut engine, p0, crumble());
    aim_at(&mut engine, p0, wall);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, living_wall()).is_some(),
        "the second Crumble destroyed the Wall"
    );
    assert_eq!(
        engine.state().players[0].life,
        24,
        "only the resolution that still had a target gained its mana value: \
         the fizzled copy added nothing"
    );
}
