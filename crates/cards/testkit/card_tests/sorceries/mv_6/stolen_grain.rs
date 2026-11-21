//! `cards/sorceries/mv_6/stolen_grain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn stolen_grain() -> CardIndex {
    card_index("dc17836d-8edf-44bd-a9ee-346d6af25dbf")
}

/// Stolen Grain (dc17836d-8edf-44bd-a9ee-346d6af25dbf) — {4}{B}{B} Sorcery:
/// "Stolen Grain deals 5 damage to target opponent or planeswalker. You gain 5
/// life."
#[test]
fn stolen_grain_drains_target_opponent_or_planeswalker_for_five() {
    let mut my_board = vec![swamp(); 6];
    my_board.push(aurochs());
    let mut engine = lk::game(
        SEED,
        &my_board,
        &[aurochs(), karn_the_great_creator()],
        &[stolen_grain()],
        &[],
    );
    let my_beast = lk::obj(&engine, P0, aurochs());
    let their_beast = lk::obj(&engine, P1, aurochs());
    let karn = lk::obj(&engine, P1, karn_the_great_creator());

    cast_from_hand(&mut engine, P0, stolen_grain());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!(player_options, vec![P1], "only opponents");
    assert_eq!(options, vec![karn], "planeswalker offered, creatures not");
    assert!(!options.contains(&my_beast) && !options.contains(&their_beast));

    lk::aim(&mut engine, &[], &[P1]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 15);
    assert_eq!(lk::life(&engine, P0), 25);

    let mut vs_pw = lk::game(
        SEED,
        &[swamp(); 6],
        &[karn_the_great_creator()],
        &[stolen_grain()],
        &[],
    );
    let karn_pw = lk::obj(&vs_pw, P1, karn_the_great_creator());
    tap_all_mana(&mut vs_pw, P0);
    lk::cast_at(&mut vs_pw, P0, stolen_grain(), &[karn_pw], &[]);
    assert_eq!(lk::zone_of(&vs_pw, karn_pw), Zone::Graveyard);
    assert_eq!(lk::life(&vs_pw, P0), 25);
}
