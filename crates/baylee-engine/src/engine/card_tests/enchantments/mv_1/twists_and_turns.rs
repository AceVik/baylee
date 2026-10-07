//! `cards/enchantments/mv_1/twists_and_turns.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Twists and Turns — "When a land you control enters, if you control seven
/// or more lands, transform this enchantment." Explore is not in the DSL and
/// the file says so, so the transform trigger is the whole of what this card
/// does here — and it is two printed words wearing one clause: **a land you
/// control** entering, and **you** controlling seven of them.
///
/// Both are only readable against a board that would fool the other reading.
/// The opponent sits on six lands of their own, so a count that forgot whose
/// permanents it was over would transform the enchantment on the sixth land
/// rather than the seventh; the sixth is played first and has to leave an
/// enchantment standing. `Condition::ControlCount` restricts to `you` before
/// its filter runs, which is why the card writes plain `Filter::LAND` and is
/// right to — this test is what says so.
#[test]
fn twists_and_turns_transforms_on_your_seventh_land_and_not_on_the_tables() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                twists_and_turns(),
            ],
        )
        .battlefield(
            1,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let enchantment = on_battlefield(&engine, p0, twists_and_turns()).expect("it is out");
    let enchantment_was = identity(&engine, enchantment);
    assert!(
        types(&engine, enchantment).contains(TypeSet::ENCHANTMENT),
        "it starts as the face it prints"
    );

    // The sixth. Eleven lands are on the table by now and six of them are
    // p0's, so a table-wide count would fire here.
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        types(&engine, enchantment).contains(TypeSet::ENCHANTMENT),
        "six is not seven — and the six across the table are not yours"
    );

    // The seventh, next turn: one land drop per turn (CR 305.2), so the
    // turn has to go round before the second Forest can be played.
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);

    let transformed = on_battlefield(&engine, p0, twists_and_turns())
        .expect("the card is still on the battlefield, as Mycoid Maze");
    assert_eq!(
        identity(&engine, transformed),
        enchantment_was,
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );
    assert!(
        types(&engine, transformed).contains(TypeSet::LAND),
        "the seventh land turns it over: Mycoid Maze is a Land — Cave"
    );
    assert!(
        !types(&engine, transformed).contains(TypeSet::ENCHANTMENT),
        "and it is no longer the enchantment — a transform is not an addition"
    );
    activate(&mut engine, p0, twists_and_turns(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "Mycoid Maze's own {{T}}: Add {{G}}, which is the back face's ability \
         and not the front's"
    );
}

/// Two lands entering at once trigger Twists and Turns twice, and the first
/// to resolve turns it over. The second is an ability of the permanent that
/// tries to transform it after it has transformed since the ability was put
/// on the stack, so it is ignored (CR 701.27f): Mycoid Maze stays Mycoid
/// Maze. Blighted Woodland's sacrifice leaves six lands and its two Forests
/// make eight, so "if you control seven or more lands" holds for both.
#[test]
fn twists_and_turns_triggered_twice_at_once_transforms_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                twists_and_turns(),
                blighted_woodland(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let enchantment = on_battlefield(&engine, p0, twists_and_turns()).expect("it is out");
    let enchantment_was = identity(&engine, enchantment);
    let woodland = on_battlefield(&engine, p0, blighted_woodland()).expect("the Woodland");

    tap_mana_except(&mut engine, p0, woodland);
    activate(&mut engine, p0, blighted_woodland(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0], options[1]],
            },
        )
        .expect("two Forests");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine
            .state()
            .object(enchantment)
            .map(|o| (o.zone, o.face_index)),
        Some((crate::zone::Zone::Battlefield, 1)),
        "turned over once, and the second trigger did not turn it back"
    );
    assert_eq!(
        identity(&engine, enchantment),
        enchantment_was,
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );
    assert_eq!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .filter(
                |e| matches!(e.event, GameEvent::Transformed { object, .. } if object == enchantment)
            )
            .count(),
        1,
        "one transform"
    );
}
