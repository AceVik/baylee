//! `cards/lands/utility/thran_quarry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thran Quarry: "At the beginning of the end step, if you control no creatures, sacrifice this land." / "{T}: Add one mana of any color."
/// Under `Coverage::Partial`, the negative creature count end-step sacrifice trigger is omitted.
/// Activating the land prompts for a color choice and adds one mana of the chosen color ({G}) to the pool.
#[test]
fn thran_quarry_taps_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(121, forest())
        .battlefield(0, &[thran_quarry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, thran_quarry()).expect("Thran Quarry deployed");
    activate(&mut engine, p0, thran_quarry(), 0);

    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Thran Quarry is Glimmervoid's twin over creatures: "At the beginning of
/// the end step, if you control no creatures, sacrifice this land." The
/// bystander is the half worth having — an artifact does not hold this one
/// up, which is what says the two cards read different filters rather than
/// sharing one.
#[test]
fn thran_quarry_sacrifices_itself_unless_a_creature_holds_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut alone = Duel::new(9211, forest())
        .battlefield(0, &[thran_quarry(), basilisk_collar()])
        .start();
    keep_mulligans(&mut alone);
    reach_main_phase(&mut alone, p0);
    reach_their_main_phase(&mut alone, p1);
    assert!(
        on_battlefield(&alone, p0, thran_quarry()).is_none(),
        "an artifact is not a creature, so the Quarry goes"
    );

    let mut held = Duel::new(9212, forest())
        .battlefield(0, &[thran_quarry(), llanowar_elves()])
        .start();
    keep_mulligans(&mut held);
    reach_main_phase(&mut held, p0);
    reach_their_main_phase(&mut held, p1);
    assert!(
        on_battlefield(&held, p0, thran_quarry()).is_some(),
        "one creature is enough"
    );
}
