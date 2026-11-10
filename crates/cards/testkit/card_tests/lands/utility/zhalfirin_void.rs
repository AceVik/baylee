//! `cards/lands/utility/zhalfirin_void.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Zhalfirin Void is a utility land under `Coverage::Implemented` that scries 1 upon entering and taps for {C}.
/// Playing the land puts it onto the battlefield untapped and places its enters-the-battlefield trigger on the stack.
/// When the trigger resolves, a scry arrangement is presented via `ArrangePrompt::Scry`.
/// After the trigger is answered, the land can be tapped to produce one colorless mana.
#[test]
fn zhalfirin_void_enters_untapped_scries_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[zhalfirin_void()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, zhalfirin_void());
    assert!(
        !entered_tapped(&engine, land),
        "Zhalfirin Void enters untapped"
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
        panic!("expected a scry arrangement, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "controller resolves the scry trigger");
    assert_eq!(
        piles,
        scry_piles(1),
        "scry 1 allows choosing 0 or 1 card to bottom"
    );
    assert_eq!(prompt, ArrangePrompt::Scry, "prompt is Scry");

    engine
        .apply(p0, look_answer(&cards, &[]))
        .expect("keeping card on top is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "land is still untapped after trigger resolves"
    );

    // A mana ability the card prints is an ordinary `(source, index)` entry
    // pressed with `ActivateAbility`; `ActivateManaAbility` is only the
    // CR 305.6 shortcut a basic land type gives. Index 1: the enters trigger
    // is the card's first ability.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("tapping for colorless mana is legal");

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "one colorless mana added to pool"
    );
    assert!(
        is_tapped(&engine, land),
        "land is tapped after activating mana ability"
    );
}
