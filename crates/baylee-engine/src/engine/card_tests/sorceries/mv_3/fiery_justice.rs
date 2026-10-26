//! `cards/sorceries/mv_3/fiery_justice.rs`, played against both target groups.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

fn fiery_justice() -> CardIndex {
    card_index("333809cb-e196-45f2-8a67-31374438e56e")
}

fn prepared() -> Engine<RegistryLookup> {
    let mut e = Duel::table(SEED, forest(), 3)
        .hand(0, &[fiery_justice()])
        .battlefield(0, &[index::WALL_OF_STONE])
        .battlefield(1, &[index::SERRA_ANGEL])
        .hand(1, &[index::UNSUMMON])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    lk::float(
        &mut e,
        P0,
        &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::White, 1),
        ],
    );
    e
}

/// The two printed instances of "target" are separate. The opponent who
/// gains life need not be the damaged player, and own creatures are targets.
#[test]
fn fiery_justice_divides_five_and_targets_one_opponent_for_life() {
    let mut e = prepared();
    let wall = lk::obj(&e, P0, index::WALL_OF_STONE);
    let angel = lk::obj(&e, P1, index::SERRA_ANGEL);
    cast_with_floating(&mut e, P0, fiery_justice());
    lk::aim(&mut e, &[wall, angel], &[P1]);
    let Pending::ChooseTargets {
        options,
        player_options,
        min,
        max,
        ..
    } = e.pending()
    else {
        panic!("separate opponent target")
    };
    assert!(options.is_empty());
    assert_eq!((*min, *max), (1, 1));
    assert_eq!(player_options.len(), 2);
    assert!(player_options.contains(&P1) && player_options.contains(&P2));
    assert!(!player_options.contains(&P0));
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0]
            }
        )
        .is_err()
    );
    lk::aim(&mut e, &[], &[P2]);
    assert!(e.apply(P0, PlayerAction::ChooseNumber(0)).is_err());
    assert!(e.apply(P0, PlayerAction::ChooseNumber(4)).is_err());
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::damage_on(&e, wall), 2);
    assert_eq!(lk::damage_on(&e, angel), 1);
    assert_eq!(lk::life(&e, P0), 20);
    assert_eq!(lk::life(&e, P1), 18);
    assert_eq!(lk::life(&e, P2), 25);
}

/// The same opponent can appear in both target groups: five damage first,
/// then five life gained. Zero damage targets still keeps the life target.
#[test]
fn fiery_justice_allows_the_same_player_in_both_groups_or_no_damage_targets() {
    for (damage, initial_life) in [(false, 20), (true, 20), (true, 3)] {
        let mut e = prepared();
        e.dev_state_mut(P0).unwrap().players[1].life = initial_life;
        e.refresh_offer();
        cast_with_floating(&mut e, P0, fiery_justice());
        lk::aim(&mut e, &[], if damage { &[P1] } else { &[] });
        lk::aim(&mut e, &[], &[P1]);
        lk::settle(&mut e);
        assert_eq!(lk::life(&e, P1), initial_life + if damage { 0 } else { 5 });
        assert_eq!(lk::life(&e, P2), 20);
    }
}

/// A damaged creature that becomes illegal loses its share; the independently
/// targeted opponent still gains five, and surviving shares do not increase.
#[test]
fn fiery_justice_preserves_life_target_when_a_damage_target_leaves() {
    let mut e = prepared();
    let wall = lk::obj(&e, P0, index::WALL_OF_STONE);
    let angel = lk::obj(&e, P1, index::SERRA_ANGEL);
    cast_with_floating(&mut e, P0, fiery_justice());
    lk::aim(&mut e, &[angel, wall], &[]);
    lk::aim(&mut e, &[], &[P2]);
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    lk::float(&mut e, P1, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut e, P1, index::UNSUMMON);
    lk::aim(&mut e, &[angel], &[]);
    lk::settle(&mut e);
    assert!(in_hand(&e, P1, index::SERRA_ANGEL).is_some());
    assert_eq!(lk::damage_on(&e, wall), 2);
    assert_eq!(lk::life(&e, P2), 25);
}
