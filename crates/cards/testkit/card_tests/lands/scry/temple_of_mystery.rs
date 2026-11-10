//! `cards/lands/scry/temple_of_mystery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Temple of Mystery` enters tapped, scries 1 upon entering, and taps for `{{G}}` or `{{U}}`
/// under `Coverage::Implemented`.
/// Playing the land enters it tapped and triggers scry 1, which asks a scry arrangement (`ArrangePrompt::Scry`).
/// Declining to bottom keeps the card on top and leaves the land tapped on the battlefield.
#[test]
fn temple_of_mystery_enters_tapped_and_scries() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(2304, forest())
        .hand(0, &[temple_of_mystery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, temple_of_mystery());
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
