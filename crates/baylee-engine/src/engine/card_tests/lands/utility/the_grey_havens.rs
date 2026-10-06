//! `cards/lands/utility/the_grey_havens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Grey Havens: "When The Grey Havens enters, scry 1." / "{T}: Add {C}." / "{T}: Add one mana of any color among legendary creature cards in your graveyard."
/// Under `Coverage::Partial`, reading colors from cards in a graveyard is unsupported.
/// Playing The Grey Havens triggers scry 1, and activating ability 1 adds one colorless mana to the pool.
#[test]
fn the_grey_havens_triggers_scry_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(137, forest())
        .hand(0, &[the_grey_havens()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, the_grey_havens());
    assert!(!entered_tapped(&engine, land));

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
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a scry arrangement, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(piles, scry_piles(1));

    engine.apply(p0, look_answer(&cards, &[])).unwrap();
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, the_grey_havens(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
