//! `cards/lands/utility/lotus_field.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lotus Field: "Hexproof" / "This land enters tapped." / "When this land enters, sacrifice two lands." / "{T}: Add three mana of any one color."
/// Playing Lotus Field enters tapped with hexproof and triggers an ETB requiring two lands to be sacrificed.
/// On the following turn, Lotus Field untaps and produces three mana of the chosen color ({U}).
#[test]
fn lotus_field_enters_tapped_sacrifices_two_lands_and_adds_three_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(119, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[lotus_field()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let f1 = all_on_battlefield(&engine, p0, forest())[0];
    let f2 = all_on_battlefield(&engine, p0, forest())[1];

    let lotus = play_land(&mut engine, p0, lotus_field());
    assert!(entered_tapped(&engine, lotus), "Lotus Field enters tapped");
    assert!(
        keywords(&engine, lotus).contains(KeywordSet::HEXPROOF),
        "Lotus Field has hexproof"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![f1] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![f2] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(all_on_battlefield(&engine, p0, forest()).len(), 0);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, lotus), "untaps on next turn");

    // Index 1: ability 0 is the enters-trigger that sacrificed the two
    // Forests above, and a trigger is never in the activatable set.
    activate(&mut engine, p0, lotus_field(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 3);
    assert!(is_tapped(&engine, lotus));
}
