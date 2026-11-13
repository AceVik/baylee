//! `cards/lands/scry/temple_of_malady.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Temple of Malady` enters tapped, scries 1 upon entering, and taps for `{{B}}` or `{{G}}`
/// under `Coverage::Implemented`.
/// Playing the land enters it tapped and triggers scry 1, which asks a scry arrangement (`ArrangePrompt::Scry`).
/// Declining to bottom keeps the card on top and leaves the land tapped on the battlefield.
#[test]
fn temple_of_malady_enters_tapped_and_scries() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(2302, forest())
        .hand(0, &[temple_of_malady()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, temple_of_malady());
    assert!(entered_tapped(&engine, land));

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
    assert_eq!(player, p0);
    assert_eq!(piles, scry_piles(1));
    assert_eq!(prompt, ArrangePrompt::Scry);

    engine.apply(p0, look_answer(&cards, &[])).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, land));
}

#[test]
fn temple_of_malady_untaps_and_taps_for_black_or_green() {
    let p0 = PlayerId::new(0);
    for (seed, color) in [(2303, ManaColor::Black), (2304, ManaColor::Green)] {
        let mut engine = Duel::new(seed, forest())
            .hand(0, &[temple_of_malady()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, temple_of_malady());
        assert!(entered_tapped(&engine, land));
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::Arrange {
                    prompt: ArrangePrompt::Scry,
                    ..
                }
            )
        });
        let Pending::Arrange { cards, .. } = engine.pending().clone() else {
            panic!("expected scry 1, got {:?}", engine.pending());
        };
        engine.apply(p0, look_answer(&cards, &[])).unwrap();

        cross_into_the_next_own_main(&mut engine, p0);
        assert!(!is_tapped(&engine, land), "the Temple untaps normally");
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 1,
                },
            )
            .expect("the printed mana ability follows the enters trigger");
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("the Temple offers its two colors: {:?}", engine.pending());
        };
        assert_eq!(options, vec![ManaColor::Black, ManaColor::Green]);
        engine.apply(p0, PlayerAction::ChooseColor(color)).unwrap();

        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.total(), 1);
        assert_eq!(pool.available(color), 1);
        assert!(is_tapped(&engine, land));
        assert!(stack_is_empty(&engine), "a mana ability uses no stack");
    }
}
