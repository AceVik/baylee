//! `cards/lands/gain/los_diablos_missile_base.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Los Diablos Missile Base prints three sentences and all three are written:
/// it enters tapped, its arrival gains its controller 1 life, and it taps for
/// {R} or {G}. The entry is played as a real land drop rather than seeded,
/// because `starting_battlefield` is a placement no replacement effect looks
/// at — a board built that way would arrive untapped whatever the card says.
/// The tap is deliberately left to the *next* turn: a land that entered tapped
/// has no `{T}` to offer before its controller's untap step, and a test that
/// pressed it right away would have skipped over that half of the card.
#[test]
fn los_diablos_missile_base_enters_tapped_gains_a_life_and_taps_for_red_or_green() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(411, forest())
        .hand(0, &[los_diablos_missile_base()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, los_diablos_missile_base());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        entered_tapped(&engine, land),
        "the printed enters-tapped replacement applied to a real land drop"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "the entry trigger gained its controller 1 life"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and it is the controller who gains it, not the table"
    );

    // Tapped, so there is nothing to activate until it untaps.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == land),
        "a land that entered tapped offers its {{T}} to nobody this turn: {:?}",
        legal.abilities
    );

    // A turn around, which is the only way an untap step happens.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");

    // Ability 0 is the entry trigger, so the printed mana ability is 1.
    activate(&mut engine, p0, los_diablos_missile_base(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "both printed colours are on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and a third would be a different card: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named"
    );
    assert_eq!(pool.available(ManaColor::Red), 0, "and not the other one");
    assert_eq!(pool.total(), 1, "one tap, one mana");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "and {{T}} was the whole price");
}
