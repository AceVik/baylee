//! `cards/instants/mv_1/inflame.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "8426bdb6-b002-4d27-9eaf-d7f8353aa7e8"
fn inflame() -> CardIndex {
    card_index("8426bdb6-b002-4d27-9eaf-d7f8353aa7e8")
}

/// Inflame ({R}): deals 2 damage to each creature dealt damage this turn
/// (both yours and opponent's), and leaves undamaged creatures untouched.
#[test]
fn inflame_damages_only_creatures_dealt_damage_this_turn() {
    let mut engine = lk::game(
        SEED,
        &[index::SERRA_ANGEL],
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[shock(), shock(), inflame()],
        &[],
    );
    let angel = lk::obj(&engine, P0, index::SERRA_ANGEL);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
    lk::cast_at(&mut engine, P0, shock(), &[giant], &[]);
    lk::cast_at(&mut engine, P0, shock(), &[angel], &[]);
    assert_eq!(lk::damage_on(&engine, giant), 2);
    assert_eq!(lk::damage_on(&engine, angel), 2);
    assert_eq!(lk::damage_on(&engine, bears), 0);

    cast_with_floating(&mut engine, P0, inflame());
    lk::settle(&mut engine);

    assert!(
        in_graveyard(&engine, P1, index::HILL_GIANT).is_some(),
        "damaged opponent creature takes 2 more and dies"
    );
    assert!(
        in_graveyard(&engine, P0, index::SERRA_ANGEL).is_some(),
        "damaged own creature also takes 2 more and dies"
    );
    assert_eq!(
        on_battlefield(&engine, P1, index::GRIZZLY_BEARS),
        Some(bears),
        "undamaged creature is untouched"
    );
    assert_eq!(lk::damage_on(&engine, bears), 0);
}

/// Damage dealt on an earlier turn does not qualify for Inflame on a new turn.
#[test]
fn inflame_ignores_damage_from_previous_turn() {
    let mut engine = lk::game(SEED, &[], &[index::SERRA_ANGEL], &[shock(), inflame()], &[]);
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, shock(), &[angel], &[]);
    assert_eq!(lk::damage_on(&engine, angel), 2);

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(lk::damage_on(&engine, angel), 0);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, inflame());
    lk::settle(&mut engine);

    assert_eq!(
        lk::damage_on(&engine, angel),
        0,
        "damage from a prior turn does not make Inflame hit the Angel"
    );
}
