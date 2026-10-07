//! `cards/creatures/mv_1/viridian_acolyte.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Viridian Acolyte — {G}, a 1/1 Elf Shaman — prints one line: "{1}, {T}:
/// Add one mana of any color."
///
/// The colour is the whole card, so the scenario reads it rather than a mana
/// count: the only land on the board is a Forest, which makes green, and the
/// mana standing in the pool afterwards is black — a colour nothing else here
/// could have produced, named out of the five the card promises and never a
/// default. The {1} is a real payment out of the green the Forest put there,
/// and the Acolyte is tapped by its own price.
///
/// The turn cycle before the activation is CR 302.6: the Acolyte is cast on
/// turn one, so its `{T}` is not payable until its controller's next turn.
#[test]
fn viridian_acolyte_taps_for_one_mana_of_the_colour_its_controller_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[viridian_acolyte()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, viridian_acolyte());
    pass_until(&mut engine, stack_is_empty);
    let acolyte = on_battlefield(&engine, p0, viridian_acolyte()).expect("the Acolyte resolved");
    assert_eq!(pt(&engine, acolyte), (1, 1), "the body the card prints");

    // A whole turn cycle, because a creature that entered this turn cannot
    // pay a `{T}` (CR 302.6) — and the pool the cast left behind empties as
    // its phase ends (CR 500.5), so the board is quiet when it comes back.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats at the start of the turn the Acolyte may tap"
    );

    // `legal.abilities` is filtered by `can_afford`, which reads the pool:
    // the Forest goes down first, and it is the whole of the `{1}`.
    tap_all_mana_but(&mut engine, p0, Some(viridian_acolyte()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest, one green"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(acolyte, 0)),
        "`{{1}}, {{T}}` is payable and the Acolyte is untapped, so its one \
         line is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&acolyte),
        "a printed mana ability is an ordinary `(source, index)` entry and \
         not the CR 305.6 shortcut (#159): {:?}",
        legal.mana_abilities
    );

    // Ability 0 is the printed line; the card has no other.
    activate(&mut engine, p0, viridian_acolyte(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colours of the game, and colourless is no colour at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the green the Forest made paid the {{1}}"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
    assert!(
        is_tapped(&engine, acolyte),
        "the Acolyte paid its own {{T}}"
    );
}
