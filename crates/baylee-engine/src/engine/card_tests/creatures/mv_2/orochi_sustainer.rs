//! `cards/creatures/mv_2/orochi_sustainer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Orochi Sustainer is a `{1}{G}` 1/2 Snake Shaman whose entire printed
/// is a single mana ability: "{T}: Add {G}". The card is thus truly played —
/// the body is read from the battlefield, and the tap happens a turn later so
/// that the creature is unambiguously no longer summoning-sick (CR 302.6) and
/// both Forests are standing again. Precisely these two standing Forests are
/// the control: “exactly one green and nothing else” is only a statement
/// about the Sustainer if no source next to it has moved — and no
/// `ChooseColor` question is asked, because the card names its color where
/// “Add one mana of any color” would have to choose (CR 605.3b: no stack).
#[test]
fn orochi_sustainer_taps_for_one_green_and_nothing_else_moves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1731, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[orochi_sustainer()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{G} from exactly the two Forests: two green tapped and two paid,
    // so the pool is empty when the creature lands — the green below cannot
    // therefore be a leftover from the calculation.
    cast_from_hand(&mut engine, p0, orochi_sustainer());
    pass_until(&mut engine, stack_is_empty);
    let snake = on_battlefield(&engine, p0, orochi_sustainer()).expect("the Sustainer resolved");
    assert_eq!(pt(&engine, snake), (1, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two Forests pay exactly the {{1}}{{G}} and leave nothing floating"
    );

    // A turn change, so that the {T} is unambiguously payable (CR 302.6) and
    // both Forests stand up again — they are the control against which the
    // green is read, and they must stand for that.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    let lands = all_on_battlefield(&engine, p0, forest());
    assert_eq!(lands.len(), 2, "both Forests are still on the table");
    assert!(
        lands.iter().all(|id| !is_tapped(&engine, *id)),
        "and both are untapped, so neither has paid into the pool below"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats on the new turn, so the tap is the only source there is"
    );

    // Ability 0 is the printed "{T}: Add {G}" — a mana ability that a card
    // prints, thus an ordinary entry in `abilities` with an index, and its
    // entire cost is its own tap symbol.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(snake, 0)),
        "an untapped Sustainer with nothing floating is a payable {{T}}: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, orochi_sustainer(), 0);
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "the card names its colour, so there is nothing to choose: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "{{T}}: Add {{G}}");
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, and nothing beside it"
    );
    assert!(
        is_tapped(&engine, snake),
        "the Sustainer paid its own {{T}}"
    );
    assert!(
        lands.iter().all(|id| !is_tapped(&engine, *id)),
        "and no Forest moved: the green came off the creature and not off a land"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
