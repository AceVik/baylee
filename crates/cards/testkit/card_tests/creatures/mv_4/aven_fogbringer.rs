//! `cards/creatures/mv_4/aven_fogbringer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aven Fogbringer — {3}{U} — a 2/1 Bird Wizard with flying and "When this
/// creature enters, return target land to its owner's hand."
///
/// The scenario is aimed at the two words that decide the card: the target is
/// *any* land, and the land goes to its **owner's** hand rather than to the
/// controller of the trigger. One board reads both — the Forest across the
/// table is on the menu, is chosen, and lands back in the hand of the seat
/// that owns it — while my own Island is still on the battlefield afterwards,
/// so a trigger that had bounced its controller's land, or a filter narrowed
/// to "you control", would each fail here. The body and the keyword are read
/// off the permanent once the trigger has resolved, which is what says the
/// entry did not cost the creature anything.
#[test]
fn aven_fogbringer_bounces_a_land_to_the_hand_of_the_seat_that_owns_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[aven_fogbringer()])
        // A land and a creature across the table: "target land" has to take
        // the first and decline the second.
        .battlefield(1, &[forest(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, island()).expect("my Island is out");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let bystander = on_battlefield(&engine, p1, quiet_creature()).expect("their creature is out");

    cast_from_hand(&mut engine, p0, aven_fogbringer());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Aven's controller chooses its own trigger");
    assert_eq!((min, max), (1, 1), "one land, and the trigger asks once");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target land\" is any land, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&bystander),
        "a creature is no land, with the Aven already on the battlefield: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Forest was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "the targeted land left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, forest()).is_some(),
        "\"to its owner's hand\": the Forest goes back to the seat that owns \
         it, not to the seat that aimed the trigger"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_none(),
        "and it is not in the Aven's controller's hand"
    );
    assert!(
        on_battlefield(&engine, p0, island()).is_some(),
        "my own land is untouched: one target, one bounce"
    );

    let bird = on_battlefield(&engine, p0, aven_fogbringer()).expect("the Aven resolved");
    assert_eq!(pt(&engine, bird), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "and the flying it prints"
    );
}
