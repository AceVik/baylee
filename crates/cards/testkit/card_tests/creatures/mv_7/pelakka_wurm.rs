//! `cards/creatures/mv_7/pelakka_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn pelakka_wurm() -> CardIndex {
    card_index("d36075c2-de66-4202-9217-b1102a2bc14b")
}

/// Pelakka Wurm — a 7/7 trampler for {4}{G}{G}{G}: "When this creature
/// enters, you gain 7 life."
#[test]
fn pelakka_wurm_gains_seven_when_it_enters() {
    let mut engine = table7(&lots(forest(), 7), &[pelakka_wurm()], &[]);
    let wurm = cast_and_resolve(&mut engine, pelakka_wurm());
    assert_body(&engine, wurm, (7, 7), KeywordSet::TRAMPLE);
    assert_eq!(life_of(&engine, 0), 27);
}

/// "When this creature dies, draw a card."
#[test]
fn pelakka_wurm_draws_a_card_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&[pelakka_wurm()], &[], &[]);
    let wurm = on_battlefield(&engine, p0, pelakka_wurm()).expect("out");
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library = library_size(&engine, p0);
    kill(&mut engine, wurm);
    assert!(in_graveyard(&engine, p0, pelakka_wurm()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1,
        "a card drawn"
    );
    assert_eq!(library_size(&engine, p0), library - 1);
}
