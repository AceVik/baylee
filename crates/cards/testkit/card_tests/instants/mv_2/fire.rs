//! `cards/instants/mv_2/fire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Ice face taps its permanent and draws exactly one card.
#[test]
fn ice_taps_a_permanent_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(23, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[fire()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");
    assert!(!is_tapped(&engine, elf), "the Elf starts untapped");

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, fire());
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let ice_slot = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Face(1)))
            .expect("Ice (face 1) is one of the options");
        engine
            .apply(p0, PlayerAction::ChooseMode(ice_slot))
            .expect("choosing the Ice face is legal");
    }

    // "Tap target permanent" — the engine asks for a target.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Ice asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "any permanent on the battlefield is a legal target: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, elf),
        "\"Tap target permanent\" — the Elf is tapped"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the spell left the hand and \"Draw a card\" put one back — net zero"
    );
    assert!(
        in_graveyard(&engine, p0, fire()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
}
