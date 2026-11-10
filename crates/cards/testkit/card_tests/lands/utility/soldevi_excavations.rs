//! `cards/lands/utility/soldevi_excavations.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pay the untapped Island entry replacement, then exercise both activated abilities.
#[test]
fn soldevi_excavations_enters_untapped_adds_colorless_blue_and_scries() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[soldevi_excavations()])
        .battlefield(0, &[soldevi_excavations(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let played = play_land(&mut engine, p0, soldevi_excavations());
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let payment = on_battlefield(&engine, p0, island()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![payment],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(payment).unwrap().zone,
        Zone::Graveyard
    );
    assert!(
        !entered_tapped(&engine, played),
        "enters untapped after sacrificing an Island"
    );

    // Ability 0 adds {{C}}{{U}}.
    activate(&mut engine, p0, soldevi_excavations(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.total(), 2);

    // Ability 1 costs {{1}}, {{T}} to Scry 1 on the second copy.
    activate(&mut engine, p0, soldevi_excavations(), 1);

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

    // Keep the scried card on top.
    engine.apply(p0, look_answer(&cards, &[])).unwrap();

    pass_until(&mut engine, stack_is_empty);

    // One of the two paid the `{{1}}`, and which one is the engine's to
    // choose: a generic pip takes whatever the pool offers, so asserting the
    // colour here would pin a decision the card does not make. What the card
    // says is that exactly one mana was spent.
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(
        pool.available(ManaColor::Colorless) + pool.available(ManaColor::Blue),
        1
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, soldevi_excavations())
            .into_iter()
            .filter(|&id| is_tapped(&engine, id))
            .count(),
        2,
        "both copies are now tapped"
    );
}
