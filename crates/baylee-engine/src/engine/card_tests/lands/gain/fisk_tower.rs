//! `cards/lands/gain/fisk_tower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fisk Tower prints three lines and all three are implemented: "This land
/// enters tapped", "When this land enters, you gain 1 life", and
/// "{T}: Add {W} or {B}". One turn cycle reads all three, and each is what
/// keeps the others honest — the tapped entry (CR 614.1c) is why the printed
/// mana line is not even offered in the turn the land arrives, and the untap
/// step on the way back is what shows the ability was withheld by the tap
/// rather than by a land that has no mana ability at all.
#[test]
fn fisk_tower_enters_tapped_gains_a_life_and_later_taps_for_white_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(2024, forest())
        .hand(0, &[fisk_tower()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real `PlayLand`, not a seeded `starting_battlefield`: the latter is a
    // placement rather than an entry and no enter modifier would be read.
    let land = play_land(&mut engine, p0, fisk_tower());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the entry trigger is a trigger: it is on the stack, not in the life total"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"When this land enters, you gain 1 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the land's controller and not to the table"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a tapped land can pay no {{T}}, so nothing it prints is offered in \
         the turn it arrived"
    );

    // The land's own untap step, which is the only thing that can offer the
    // mana line again.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");

    // Ability 0 is the entry trigger; the printed mana ability is ability 1.
    activate(&mut engine, p0, fisk_tower(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{B}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the color");
    assert_eq!(options.len(), 2, "two colors and nothing else: {options:?}");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "\"Add {{W}} or {{B}}\": {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::White), 0, "and not the other one");
    assert_eq!(pool.total(), 1, "one mana off one tap");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
