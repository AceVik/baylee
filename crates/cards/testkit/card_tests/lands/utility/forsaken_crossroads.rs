//! `cards/lands/utility/forsaken_crossroads.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Forsaken Crossroads: "Forsaken Crossroads enters the battlefield tapped." / "As Forsaken Crossroads enters the battlefield, choose a color." / "When Forsaken Crossroads enters the battlefield, scry 1..." / "{T}: Add one mana of the chosen color."
/// Under `Coverage::Partial`, the non-starting player untap replacement is omitted.
/// Entering prompts for a color and scries 1 while entering tapped, and after untapping it taps for the chosen color.
#[test]
fn forsaken_crossroads_enters_tapped_chooses_color_and_scries() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(224, forest())
        .hand(0, &[forsaken_crossroads()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, forsaken_crossroads()).expect("crossroads in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice on entry, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    // The enters trigger scries 1, and it is mandatory here: the "you may
    // untap instead" half is the clause `Coverage::Partial` names.
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
        unreachable!("the predicate just matched")
    };
    engine.apply(p0, look_answer(&cards, &[])).unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(entered_tapped(&engine, card));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, card));

    activate(&mut engine, p0, forsaken_crossroads(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, card));
}
