//! `cards/lands/utility/stardew_valley.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Stardew Valley` prints `{{T}}: Add {{C}}.`, `{{2}}, {{T}}, Tap an untapped creature you control: Create a Food token.`, and `{{3}}, {{T}}: Choose target permanent you control. Draw a card, then another player of your choice may gain control of that permanent. Activate only as a sorcery.`
///
/// Under `Coverage::Partial`, ability 1 requires floating `{{2}}`, tapping `Stardew Valley`, and tapping an untapped creature you control via `ChoicePrompt::CostTap`, which rejects opponent-controlled creatures.
/// Resolving ability 1 creates one Food token and leaves both `Stardew Valley` and the chosen creature tapped.
#[test]
fn stardew_valley_taps_controlled_creature_to_create_food_token() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[stardew_valley(), forest(), forest(), young_wolf()])
        .battlefield(1, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let valley = on_battlefield(&engine, p0, stardew_valley()).expect("valley on battlefield");
    let my_wolf = on_battlefield(&engine, p0, young_wolf()).expect("my wolf on battlefield");
    let their_wolf = on_battlefield(&engine, p1, young_wolf()).expect("their wolf on battlefield");

    assert!(!is_tapped(&engine, my_wolf));

    // Float {{2}} from Forests while keeping Stardew Valley untapped.
    tap_mana_except(&mut engine, p0, valley);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!is_tapped(&engine, valley));

    activate(&mut engine, p0, stardew_valley(), 1);

    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected CostTap prompt, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::CostTap);
    assert_eq!((min, max), (1, 1));
    assert!(
        options.contains(&my_wolf),
        "controlled creature is offered as cost"
    );
    assert!(
        !options.contains(&their_wolf),
        "opponent creature cannot be tapped for your cost"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_wolf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, valley));
    assert!(is_tapped(&engine, my_wolf));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "created one Food token");
}
