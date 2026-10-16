//! `cards/creatures/artifacts/mv_3/parasitic_strix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn parasitic_strix() -> CardIndex {
    card_index("975b4e0f-1133-4ffa-a6b1-d319d248dcde")
}

/// Parasitic Strix — "When this creature enters, if you control a black
/// permanent, target player loses 2 life and you gain 2 life." With a black
/// permanent out the trigger asks for a player.
#[test]
fn parasitic_strix_drains_a_target_player_when_you_control_a_black_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // Etched Familiar is black.
    let black = card_index("e2bc064e-4739-4eb1-ba44-1fd92b2829d7");
    let mut engine = main_with(&[parasitic_strix()], &[island(), island(), island(), black]);

    cast_from_hand(&mut engine, p0, parasitic_strix());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let offered = aim_at_player(&mut engine, p0, p1);
    assert!(offered.contains(&p0) && offered.contains(&p1), "any player");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, 1), 18, "the target lost 2");
    assert_eq!(life_of(&engine, 0), 22, "and its controller gained 2");
    let strix = on_battlefield(&engine, p0, parasitic_strix()).expect("it resolved");
    assert_eq!(pt(&engine, strix), (2, 2));
    assert!(keywords(&engine, strix).contains(KeywordSet::FLYING));
}

/// Without a black permanent the condition is false: no target is asked for
/// and nobody's life changes (the Strix itself is blue and colorless-less).
#[test]
fn parasitic_strix_does_nothing_without_a_black_permanent() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[parasitic_strix()], &[island(), island(), island()]);

    cast_and_settle(&mut engine, parasitic_strix());

    assert!(on_battlefield(&engine, p0, parasitic_strix()).is_some());
    assert_eq!(life_of(&engine, 0), 20);
    assert_eq!(life_of(&engine, 1), 20);
}
