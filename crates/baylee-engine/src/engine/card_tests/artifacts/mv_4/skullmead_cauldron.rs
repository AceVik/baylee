//! `cards/artifacts/mv_4/skullmead_cauldron.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn skullmead_cauldron() -> CardIndex {
    card_index("b6864a26-9c13-4c6d-b0e0-d5b5ed9864e8")
}

/// "{T}: You gain 1 life."
#[test]
fn tapping_gains_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skullmead_cauldron()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cauldron = on_battlefield(&engine, p0, skullmead_cauldron()).unwrap();
    activate(&mut engine, p0, skullmead_cauldron(), 0);
    assert!(is_tapped(&engine, cauldron));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 21);
}

/// "{T}, Discard a card: You gain 3 life." The discard is a cost and is asked
/// for; with an empty hand the ability is not offered.
#[test]
fn discarding_a_card_gains_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skullmead_cauldron()])
        .hand(0, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cauldron = on_battlefield(&engine, p0, skullmead_cauldron()).unwrap();
    activate(&mut engine, p0, skullmead_cauldron(), 1);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("which card to discard, got {:?}", engine.pending())
    };
    let card = in_hand(&engine, p0, plains()).unwrap();
    assert_eq!(options, vec![card]);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![card] })
        .expect("a listed card");
    assert!(is_tapped(&engine, cauldron));
    assert!(in_graveyard(&engine, p0, plains()).is_some(), "discarded as a cost");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 23);
}

#[test]
fn the_discarding_line_needs_a_card_to_discard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skullmead_cauldron()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cauldron = on_battlefield(&engine, p0, skullmead_cauldron()).unwrap();
    assert!(!priority_offer(&engine).abilities.contains(&(cauldron, 1)));
    assert!(priority_offer(&engine).abilities.contains(&(cauldron, 0)));
}
