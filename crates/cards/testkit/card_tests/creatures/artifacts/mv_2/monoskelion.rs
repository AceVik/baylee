//! `cards/creatures/artifacts/mv_2/monoskelion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn monoskelion() -> CardIndex {
    card_index("c16d79d9-c159-421f-90a2-13313a1de5f6")
}

/// Monoskelion: a 1/1 that "enters with a +1/+1 counter on it" — a 2/2 once it
/// is on the battlefield.
#[test]
fn monoskelion_enters_with_a_plus_one_counter() {
    let mut engine = table(&[monoskelion()], &[forest(), forest()], &[]);
    let mono = cast_and_land(&mut engine, monoskelion());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, mono, baylee_cards_dsl::CounterKind::P1P1),
        1
    );
    assert_eq!(pt(&engine, mono), (2, 2));
}

/// "{1}, Remove a +1/+1 counter from this creature: It deals 1 damage to any
/// target." Cast off four lands (two left floating), it pings the opponent
/// for one, loses the counter, and cannot do it again without one.
#[test]
fn monoskelion_removes_its_counter_to_deal_one_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[monoskelion()],
        &[forest(), forest(), forest(), forest()],
        &[],
    );
    let mono = cast_and_land(&mut engine, monoskelion());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, monoskelion(), 0);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target question, got {:?}", engine.pending())
    };
    assert!(player_options.contains(&p1), "any target: a player");
    assert!(options.contains(&mono), "any target: a creature");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent is a legal target");
    assert_eq!(
        counters_on(&engine, mono, baylee_cards_dsl::CounterKind::P1P1),
        0,
        "the counter was a cost"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 1), 19);
    assert_eq!(pt(&engine, mono), (1, 1));
    assert!(
        !priority_offer(&engine).abilities.contains(&(mono, 0)),
        "no counter left to remove"
    );
}
