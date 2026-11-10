//! `cards/lands/utility/rumble_arena.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Rumble Arena` enters untapped under `Coverage::Implemented`, has vigilance, triggers scry 1 on entry, and filters mana.
/// Playing the land from hand puts it onto the battlefield untapped with vigilance.
/// Its enters-the-battlefield trigger asks a scry arrangement via `ArrangePrompt::Scry`.
/// After scrying, its second mana ability spends floating mana and taps to produce a chosen mana color.
#[test]
fn rumble_arena_enters_untapped_scries_and_filters_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[rumble_arena()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let arena = play_land(&mut engine, p0, rumble_arena());
    assert!(
        !entered_tapped(&engine, arena),
        "Rumble Arena enters untapped"
    );
    assert!(
        keywords_of(&engine, arena).contains(KeywordSet::VIGILANCE),
        "Rumble Arena has vigilance"
    );

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Scry,
                ..
            }
        )
    });

    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("expected a scry arrangement, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "controller chooses scry");
    assert_eq!(
        piles,
        scry_piles(1),
        "scry 1 allows choosing up to one card"
    );
    assert_eq!(prompt, ArrangePrompt::Scry);

    engine
        .apply(p0, look_answer(&cards, &[]))
        .expect("keeping on top is legal");
    pass_until(&mut engine, stack_is_empty);

    // Tap the Forest for green mana while keeping Rumble Arena untapped.
    tap_all_mana_but(&mut engine, p0, Some(rumble_arena()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green mana floating from the Forest"
    );

    // Ability 0 is ETB trigger, ability 1 is {T}: Add {C}, ability 2 is {1}, {T}: Add any color.
    activate(&mut engine, p0, rumble_arena(), 2);
    let Pending::ChooseColor {
        player: color_player,
        options,
    } = engine.pending().clone()
    else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(color_player, p0, "activating player chooses color");
    assert_eq!(options.len(), 5, "offers all five mana colors");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("choosing red is legal");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "one red mana produced by Rumble Arena"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "floating green mana spent to pay the generic cost"
    );
    assert_eq!(
        pool.total(),
        1,
        "only the produced mana remains in the pool"
    );
    assert!(
        is_tapped(&engine, arena),
        "Rumble Arena tapped to activate its ability"
    );
}
