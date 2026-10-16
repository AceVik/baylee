//! `cards/creatures/artifacts/mv_3/walking_archive.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn walking_archive() -> CardIndex {
    card_index("e2998b41-0822-4e9c-b8b1-f47c400176e9")
}

/// Walking Archive — "This creature enters with a +1/+1 counter on it": cast,
/// it is a 2/2 defender with one counter.
#[test]
fn walking_archive_enters_with_a_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[walking_archive()], &[forest(), forest(), forest()]);

    cast_and_settle(&mut engine, walking_archive());

    let archive = on_battlefield(&engine, p0, walking_archive()).expect("it resolved");
    assert_eq!(counters_on(&engine, archive, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, archive), (2, 2));
    assert!(keywords(&engine, archive).contains(KeywordSet::DEFENDER));
}

/// "At the beginning of each player's upkeep, that player draws a card for
/// each +1/+1 counter on this creature": with one counter, seat 1 draws one
/// extra card in its upkeep on top of its draw step.
#[test]
fn walking_archive_draws_the_upkeep_player_a_card_per_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[walking_archive()], &[forest(), forest(), forest()]);
    cast_and_settle(&mut engine, walking_archive());
    let archive = on_battlefield(&engine, p0, walking_archive()).expect("it resolved");
    let counters = u16::try_from(counters_on(&engine, archive, CounterKind::P1P1)).unwrap();
    let before = library_of(&engine, PlayerId::new(1)).len();

    let after = p1_library_at_its_main(&mut engine);

    assert_eq!(
        before - after,
        1 + usize::from(counters),
        "the draw step and one card per counter"
    );
}

/// "{2}{W}{U}: Put a +1/+1 counter on this creature." Paid, the Archive
/// grows, and the next upkeep draws for it.
#[test]
fn walking_archive_pays_to_add_a_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(
        &[],
        &[walking_archive(), plains(), island(), forest(), forest()],
    );
    let archive = on_battlefield(&engine, p0, walking_archive()).expect("seated");
    let start = counters_on(&engine, archive, CounterKind::P1P1);
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);

    activate(&mut engine, p0, walking_archive(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, archive, CounterKind::P1P1), start + 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
