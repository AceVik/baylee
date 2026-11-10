//! `cards/lands/lotus_vale.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lotus Vale is `Coverage::Partial`: the entry replacement (sacrifice two
/// untapped lands or this goes to the graveyard) has no `EnterModifier` and is
/// not implemented — the land enters normally.  What is implemented: `{T}: Add
/// three mana of any one color` (`Effect::mana_choice_dynamic`, one choice
/// pick for the whole amount).
///
/// The card says "any one color" — one colour is chosen and three mana of
/// that colour arrive.  This is not `mana_choice` (which picks once per mana)
/// but `mana_choice_dynamic` (one pick for the whole fixed amount).  Activating
/// it produces a `ChooseColor` question, and the three mana of the chosen
/// colour land in the pool at once with no stack (CR 605.3b).
///
/// A bystander colour proves the amount is exactly 3 and nothing leaked
/// into the other slot.
#[test]
fn lotus_vale_adds_three_mana_of_one_chosen_colour() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(53, forest())
        .battlefield(0, &[lotus_vale()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vale = on_battlefield(&engine, p0, lotus_vale()).expect("the Vale is out");

    // Ability 0 is the mana ability.  It is a printed activation, so it
    // appears in legal.abilities, not legal.mana_abilities.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(vale, 0)),
        "the mana ability is in legal.abilities: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&vale),
        "the Vale prints no basic land type: not a CR 305.6 shortcut"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: vale,
                ability_index: 0,
            },
        )
        .expect("the tap is the whole cost");

    // The resolution immediately asks which colour (mana ability, no stack).
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseColor for the colour choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options.len(), 5, "all five colours are on offer");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        3,
        "three blue from the choice"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "\\\"any one color\\\" is one pick, not three picks: no green leaked in"
    );
    assert_eq!(pool.total(), 3, "exactly three mana total");
    assert!(
        stack_is_empty(&engine),
        "a mana ability never uses the stack (CR 605.3b)"
    );
    assert!(is_tapped(&engine, vale), "the {{T}} tapped the Vale");
}
