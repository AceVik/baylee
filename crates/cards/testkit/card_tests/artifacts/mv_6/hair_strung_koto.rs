//! `cards/artifacts/mv_6/hair_strung_koto.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Hair-Strung Koto` is an artifact costing `{6}` under `Coverage::Implemented`.
/// It prints "Tap an untapped creature you control: Target player mills a card."
/// Under CR 601.2c and CR 601.2h, activating the ability prompts for the target player first
/// with `Pending::ChoosePlayer`, and then prompts with `Pending::ChooseCards` carrying `ChoicePrompt::CostTap`
/// to tap an untapped creature you control (such as `Llanowar Elves`). The target player mills one card.
#[test]
fn hair_strung_koto_taps_untapped_creature_to_mill_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hair_strung_koto(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves())
        .expect("Llanowar Elves is on the battlefield");
    assert!(!is_tapped(&engine, elf), "elf starts untapped");
    let p1_library_before = library_size(&engine, p1);

    activate(&mut engine, p0, hair_strung_koto(), 0);

    // An activated ability names a player through the ordinary target
    // question, with the seats in `player_options` and no object at all —
    // unlike a spell or a loyalty ability, which ask `ChoosePlayer`.
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the target question for Hair-Strung Koto, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the activator names the target player");
    assert!(
        options.is_empty(),
        "no object is a legal target: {options:?}"
    );
    assert!(
        player_options.contains(&p1),
        "the opponent is a legal target: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    let Pending::ChooseCards {
        prompt,
        options: tap_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected CostTap prompt for Hair-Strung Koto, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(prompt, ChoicePrompt::CostTap, "prompt is CostTap");
    assert!(
        tap_options.contains(&elf),
        "Llanowar Elves is offered to tap: {tap_options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, elf),
        "creature is tapped from paying the activation cost"
    );
    assert_eq!(
        library_size(&engine, p1),
        p1_library_before - 1,
        "target player milled one card"
    );
}
