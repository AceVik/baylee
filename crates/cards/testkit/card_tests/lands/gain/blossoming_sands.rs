//! `cards/lands/gain/blossoming_sands.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blossoming Sands prints three lines and one scenario reads all of them:
/// it enters tapped, it gains its controller 1 life as it arrives, and it
/// taps for {G} or {W}. The land has to be *played* rather than seeded with
/// `starting_battlefield`, because that path places a permanent with
/// `Cause::Setup` and no replacement effect looks at it — a land seeded
/// there stands untapped whatever `EnterModifier::Tapped` says, so the first
/// printed line would be asserted off a board that never applied it. The
/// life is read off both seats so a rule that drained the opponent cannot
/// pass, and the mana line needs a turn of its own: a land that entered
/// tapped gives nothing until its controller's next untap step (CR 502.1),
/// and until then its `{T}` is not even on the offer.
#[test]
fn blossoming_sands_enters_tapped_gains_a_life_and_taps_for_green_or_white() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[blossoming_sands()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine_before = engine.state().players[0].life;
    let theirs_before = engine.state().players[1].life;
    let land = play_land(&mut engine, p0, blossoming_sands());
    pass_until(&mut engine, |e| {
        e.state().players[0].life == mine_before + 1
    });

    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — a `starting_battlefield` placement \
         would have left it standing and this would have proved nothing"
    );
    assert_eq!(
        engine.state().players[0].life,
        mine_before + 1,
        "\"When this land enters, you gain 1 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        theirs_before,
        "the life belongs to the land's controller and not to the table"
    );

    // A whole turn cycle, because the land came in tapped and its untap step
    // is the only thing that can stand it back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran, which is what makes the {{T}} below payable at all"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating before the land taps itself"
    );

    // Ability 1 is the mana ability; ability 0 is the enters trigger.
    activate(&mut engine, p0, blossoming_sands(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the color");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::White),
        "both halves of the printed mana line are on the menu: {options:?}"
    );
    assert_eq!(options.len(), 2, "and nothing else is: {options:?}");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "and not the other half"
    );
    assert_eq!(pool.total(), 1, "one land tapped, one mana");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
