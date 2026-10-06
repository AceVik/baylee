//! `cards/lands/horizon/horizon_of_progress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Horizon of Progress is `Coverage::Partial`: the `{3}, {T}` land-from-hand
/// ability has no `Effect` and is not implemented.  The two implemented
/// abilities are the Reflecting Pool–style mana ability (`{T}, Pay 1 life:
/// Add one mana of any type that a land you control could produce`) and the
/// `{1}, {T}, Sacrifice this land: Draw a card`.
///
/// The mana ability has a cost beyond just `{T}` (`PayLife(1)`), so
/// `tap_all_mana` will not press it.  This test activates it directly, and
/// the board beside it is a **Forest and an Island** rather than two
/// Forests: "any type that a land you control could produce" is a choice
/// only where more than one type is producible, and over two Forests the
/// engine has one answer and asks nothing. A board of two Forests reads as
/// a card that never asks, which is what this test first claimed and what
/// the engine rightly refused.  The draw ability is then proved on a fresh
/// board in its own game: activating it draws a card, the land leaves the
/// battlefield, and ends up in the graveyard.
#[test]
fn horizon_of_progress_taps_for_land_color_at_one_life_and_draws_for_one_generic() {
    let p0 = PlayerId::new(0);

    // ── Ability 0: {T}, Pay 1 life: Add one mana of any type a land you control could produce.
    // A Forest and an Island beside it, so {G} and {U} are both producible
    // and the choice is a real one.
    {
        let mut engine = Duel::new(51, forest())
            .battlefield(0, &[horizon_of_progress(), forest(), island()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let horizon =
            on_battlefield(&engine, p0, horizon_of_progress()).expect("the Horizon is out");
        let life_before = engine.state().players[0].life;

        // Ability 0 is the mana ability; it costs {T} and 1 life.
        // It is in legal.abilities because it is a printed activation.
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert!(
            legal.abilities.contains(&(horizon, 0)),
            "the Horizon's mana ability is offered in legal.abilities: {:?}",
            legal.abilities
        );

        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: horizon,
                    ability_index: 0,
                },
            )
            .expect("the Horizon has two Forests beside it");

        // The mana ability asks which colour (any type a land you control could produce).
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "expected a colour choice from the land-colour mana ability, got {:?}",
                engine.pending()
            )
        };
        assert!(
            options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
            "the Forest and the Island are both types a land you control \
             could produce: {options:?}"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
            .unwrap();

        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Green),
            1,
            "one {{G}} in the pool"
        );
        assert_eq!(
            engine.state().players[0].life,
            life_before - 1,
            "the life payment was taken"
        );
        assert!(is_tapped(&engine, horizon), "the {{T}} tapped the land");
        assert!(
            stack_is_empty(&engine),
            "a mana ability never uses the stack (CR 605.3b)"
        );
    }

    // ── Ability 1: {1}, {T}, Sacrifice this land: Draw a card.
    {
        let mut engine = Duel::new(52, forest())
            .battlefield(0, &[horizon_of_progress(), forest()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        // Tap the Forest for mana (the {1} generic), leave the Horizon untapped.
        let horizon =
            on_battlefield(&engine, p0, horizon_of_progress()).expect("the Horizon is out");
        tap_mana_except(&mut engine, p0, horizon);

        let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

        // Ability index 1 is {1}, {T}, Sacrifice this land: Draw a card.
        activate(&mut engine, p0, horizon_of_progress(), 1);
        pass_until(&mut engine, stack_is_empty);

        assert_eq!(
            engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
            hand_before + 1,
            "the draw put one card into hand"
        );
        assert!(
            on_battlefield(&engine, p0, horizon_of_progress()).is_none(),
            "the Horizon sacrificed itself as part of the cost"
        );
        assert!(
            in_graveyard(&engine, p0, horizon_of_progress()).is_some(),
            "a sacrificed permanent goes to its owner's graveyard"
        );
    }
}
