//! `cards/lands/scry/witherbloom_campus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Witherbloom Campus` enters tapped, taps for `{{B}}` or `{{G}}`, and scries 1 for `{{4}}, {{T}}`
/// under `Coverage::Implemented`.
/// After entering tapped and untapping on the next turn, floating four mana from basic lands
/// activates its scry ability and asks a scry arrangement (`ArrangePrompt::Scry`).
#[test]
fn witherbloom_campus_enters_tapped_and_scries() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(2308, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[witherbloom_campus()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, witherbloom_campus());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    assert!(!is_tapped(&engine, land));

    tap_all_mana_but(&mut engine, p0, Some(witherbloom_campus()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);

    activate(&mut engine, p0, witherbloom_campus(), 1);

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
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
