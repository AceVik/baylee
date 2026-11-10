//! `cards/lands/check/clifftop_retreat.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Clifftop Retreat prints "This land enters tapped unless you control a
/// Mountain or a Plains" and "{T}: Add {R} or {W}", so the check is half the
/// card and the mana line the other half. One game plays both sides of the
/// condition: the first copy lands on an empty board and arrives tapped, and
/// the second follows two of its controller's turns later, once a Mountain
/// stands, and arrives ready. The copy that arrived tapped also tells the
/// negative the cheap way — its `{T}` is not offered at all that turn — while
/// only the copy that arrived untapped can show that "Add {R} or {W}" is a
/// real question with exactly two colors in it.
#[test]
fn clifftop_retreat_enters_tapped_unless_you_control_a_mountain_or_plains_and_taps_for_either_color()
 {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[clifftop_retreat(), clifftop_retreat(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    // An empty board satisfies nothing the card asks about: neither the land
    // being played nor anything else under its controller is a Mountain or a
    // Plains.
    let first = play_land(&mut engine, p0, clifftop_retreat());
    assert!(
        entered_tapped(&engine, first),
        "no Mountain and no Plains, so the check fails and the land arrives tapped"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == first),
        "a land that came in tapped has no {{T}} to pay this turn, so it is not offered"
    );

    // A Mountain, played the ordinary way — the control for the reading
    // above, because a board on which nothing ever arrives untapped would
    // pass that assertion just as well.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let hill = play_land(&mut engine, p0, mountain());
    assert!(
        !entered_tapped(&engine, hill),
        "a basic Mountain is an untapped permanent, so the tapped status above is a reading and not a default"
    );

    // The same card again, on a board that now holds what the check asks for.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let second = play_land(&mut engine, p0, clifftop_retreat());
    assert!(
        !entered_tapped(&engine, second),
        "a Mountain is on the battlefield, so the check holds and the land arrives ready"
    );

    // And being ready is worth something only if the mana line it prints is
    // real, so the second sentence is played off the land itself.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(source, _)| *source == second)
        .expect("the copy that entered untapped holds its {T} right away");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating, so what this one tap makes is the whole of the pool"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("the mana line is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the land's controller names the color");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
        "the red of the Mountain and the white of a Plains, and nothing else: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "a third pile would be a third color, and a colorless one is no color at all (CR 105.4): {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the two it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::Red), 0, "and not the other one");
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, and the Mountain beside it never moved"
    );
    assert!(is_tapped(&engine, second), "the Retreat paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "a mana ability uses no stack (CR 605.3b)"
    );
}
