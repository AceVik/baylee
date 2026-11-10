//! `cards/lands/gain/tcri_building.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// TCRI Building prints three sentences and each needs a different reading.
/// "Enters tapped" is only visible on the permanent's status, "when this land
/// enters, you gain 1 life" is only visible on the life total after the
/// trigger has *resolved* rather than merely been collected, and "{T}: Add {U}
/// or {R}" is only visible in the colour question the tap raises — on a board
/// whose only other mana is the Forest the duel's filler deck is built from.
/// The mana line is read a turn later, because a land that arrives tapped has
/// nothing to spend in the turn it lands.
#[test]
fn tcri_building_enters_tapped_gains_a_life_and_taps_for_blue_or_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(8817, forest())
        .hand(0, &[tcri_building()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let their_life_before = engine.state().players[1].life;
    let land = play_land(&mut engine, p0, tcri_building());
    assert!(
        entered_tapped(&engine, land),
        "the printed entry replacement still puts it in tapped"
    );

    // The enters trigger is on the stack the moment the land is; the life
    // total is the only place its resolution is visible.
    pass_until(&mut engine, |e| {
        e.state().players[0].life == life_before + 1
    });
    assert_eq!(
        engine.state().players[1].life,
        their_life_before,
        "\"you gain 1 life\" is the land's controller and not the table"
    );

    // A land that enters tapped has no {{T}} in the turn it lands, so the
    // mana line is read once its own untap step has stood it back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step gave it back, which is what the activation below needs"
    );

    // Ability 1 is the mana line; ability 0 is the enters trigger.
    activate(&mut engine, p0, tcri_building(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`{{U}} or {{R}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "both printed colours are on the menu: {options:?}"
    );
    assert_eq!(options.len(), 2, "and no third colour: {options:?}");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::Red), 0, "and not the other one");
    assert_eq!(pool.total(), 1, "one source, one mana");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
