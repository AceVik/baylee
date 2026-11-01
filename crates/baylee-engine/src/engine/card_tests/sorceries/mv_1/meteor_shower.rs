//! `cards/sorceries/mv_1/meteor_shower.rs`, played against its X-plus-one division.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn meteor_shower() -> CardIndex {
    card_index("d2110083-b30f-43f1-9352-7536f5e19d3c")
}

fn prepared(x: u32, responses: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut e = lk::game(
        SEED,
        &[index::WALL_OF_STONE],
        &[index::SERRA_ANGEL],
        &[meteor_shower()],
        responses,
    );
    lk::float(&mut e, P0, &[(ManaColor::Red, 2 * x + 1)]);
    e
}

/// {X}{X}{R} pays twice X, while the distributed damage is X plus one:
/// X=2 pays five mana and deals 1 + 1 + 1 across a creature and both players.
#[test]
fn meteor_shower_pays_double_x_and_divides_x_plus_one() {
    let mut e = prepared(2, &[]);
    let wall = lk::obj(&e, P0, index::WALL_OF_STONE);
    cast_with_floating(&mut e, P0, meteor_shower());
    let Pending::ChooseNumber { min, max, .. } = e.pending() else {
        panic!("X")
    };
    assert_eq!((*min, *max), (0, 2));
    assert!(e.apply(P0, PlayerAction::ChooseNumber(3)).is_err());
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseTargets {
        min,
        max,
        options,
        player_options,
        ..
    } = e.pending()
    else {
        panic!("targets")
    };
    assert_eq!((*min, *max), (0, 3));
    assert!(options.contains(&wall));
    assert!(player_options.contains(&P0) && player_options.contains(&P1));
    lk::aim(&mut e, &[wall], &[P0, P1]);
    assert!(e.apply(P0, PlayerAction::ChooseNumber(0)).is_err());
    assert!(e.apply(P0, PlayerAction::ChooseNumber(2)).is_err());
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    assert_eq!(lk::pool_total(&e, P0), 0);
    lk::settle(&mut e);
    assert_eq!(lk::damage_on(&e, wall), 1);
    assert_eq!(lk::life(&e, P0), 19);
    assert_eq!(lk::life(&e, P1), 19);
}

/// At X=0 the spell still deals one, and may name one target. "Any number"
/// also permits no targets at any X; no impossible division prompt follows.
#[test]
fn meteor_shower_x_zero_deals_one_and_any_x_can_choose_no_targets() {
    for (x, targeted) in [(0, true), (0, false), (2, false)] {
        let mut e = prepared(x, &[]);
        cast_with_floating(&mut e, P0, meteor_shower());
        e.apply(P0, PlayerAction::ChooseNumber(x)).unwrap();
        lk::aim(&mut e, &[], if targeted { &[P1] } else { &[] });
        assert!(matches!(e.pending(), Pending::Priority { .. }));
        lk::settle(&mut e);
        assert_eq!(lk::life(&e, P1), if targeted { 19 } else { 20 });
        assert!(in_graveyard(&e, P0, meteor_shower()).is_some());
    }
}

/// A disappearing target loses its announced share; it does not increase
/// damage to the remaining player.
#[test]
fn meteor_shower_does_not_redistribute_damage_when_a_target_leaves() {
    let mut e = prepared(2, &[index::UNSUMMON]);
    let angel = lk::obj(&e, P1, index::SERRA_ANGEL);
    cast_with_floating(&mut e, P0, meteor_shower());
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    lk::aim(&mut e, &[angel], &[P1]);
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    lk::float(&mut e, P1, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut e, P1, index::UNSUMMON);
    lk::aim(&mut e, &[angel], &[]);
    lk::settle(&mut e);
    assert!(in_hand(&e, P1, index::SERRA_ANGEL).is_some());
    assert_eq!(lk::life(&e, P1), 19);
}
