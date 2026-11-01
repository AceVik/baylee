//! `cards/instants/mv_3/fire_covenant.rs`, played against both Oracle sentences.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::NumberPrompt;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fire_covenant() -> CardIndex {
    card_index("025939a0-424a-41bf-8fc9-2ef9ea5485f7")
}

fn prepared(
    mine: &[CardIndex],
    theirs: &[CardIndex],
    responses: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = lk::game(SEED, mine, theirs, &[fire_covenant()], responses);
    lk::float(
        &mut e,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );
    e
}

fn priority(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
}

fn announce(e: &mut Engine<RegistryLookup>, x: u32) {
    cast_with_floating(e, P0, fire_covenant());
    assert!(matches!(
        e.pending(),
        Pending::ChooseNumber {
            min: 0,
            max: 20,
            reason: NumberPrompt::X,
            ..
        }
    ));
    e.apply(P0, PlayerAction::ChooseNumber(x)).unwrap();
}

fn choose(e: &mut Engine<RegistryLookup>, objects: &[ObjectId]) {
    lk::aim(e, objects, &[]);
}

/// Resolve real damage choices too: the affected player applies the single
/// prevention/redirection shield and allocates its full capacity.
fn finish(e: &mut Engine<RegistryLookup>) {
    for _ in 0..50 {
        if stack_is_empty(e) {
            return;
        }
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseDamageEffect {
                player,
                choice,
                options,
                ..
            } => {
                assert_eq!(options.len(), 1);
                e.apply(
                    player,
                    PlayerAction::ChooseDamageEffect {
                        choice,
                        effect: options[0].id,
                    },
                )
                .unwrap();
            }
            Pending::AllocatePrevention {
                player,
                choice,
                damage,
                total,
                ..
            } => {
                assert_eq!(damage.len(), 1);
                e.apply(
                    player,
                    PlayerAction::AllocatePrevention {
                        choice,
                        allocation: vec![(damage[0].id, total)],
                    },
                )
                .unwrap();
            }
            pending => panic!("unexpected resolution question: {pending:?}"),
        }
    }
    panic!("Fire Covenant did not finish");
}

/// X is chosen independently of the fixed {1}{B}{R}; both controllers'
/// creatures are legal, while lands and players are not. The caster divides
/// five as 3 + 1 + 1, rather than the former fixed one-point approximation.
#[test]
fn fire_covenant_divides_chosen_x_among_any_number_of_creatures() {
    let mut e = prepared(
        &[index::WALL_OF_STONE, index::FOREST],
        &[index::SERRA_ANGEL, index::LLANOWAR_ELVES],
        &[],
    );
    let wall = lk::obj(&e, P0, index::WALL_OF_STONE);
    let land = lk::obj(&e, P0, index::FOREST);
    let angel = lk::obj(&e, P1, index::SERRA_ANGEL);
    let elves = lk::obj(&e, P1, index::LLANOWAR_ELVES);
    announce(&mut e, 5);
    let Pending::ChooseTargets {
        options,
        player_options,
        min,
        max,
        ..
    } = e.pending()
    else {
        panic!("targets")
    };
    assert_eq!((*min, *max), (0, 3));
    assert_eq!(options.len(), 3);
    assert!(options.contains(&wall) && options.contains(&angel) && options.contains(&elves));
    assert!(!options.contains(&land));
    assert!(player_options.is_empty());
    for (objects, players) in [
        (vec![land], vec![]),
        (vec![], vec![P1]),
        (vec![wall, wall], vec![]),
    ] {
        assert!(
            e.apply(P0, PlayerAction::ChooseTargets { objects, players })
                .is_err()
        );
    }
    choose(&mut e, &[wall, angel, elves]);
    assert_eq!(lk::life(&e, P0), 20, "shares precede payment");
    assert!(e.apply(P0, PlayerAction::ChooseNumber(0)).is_err());
    assert!(
        e.apply(P0, PlayerAction::ChooseNumber(4)).is_err(),
        "reserve one per remaining target"
    );
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    assert_eq!(lk::life(&e, P0), 15, "X life paid before anyone responds");
    assert_eq!(
        lk::pool_total(&e, P0),
        0,
        "only the printed three mana was needed"
    );
    finish(&mut e);
    assert_eq!(lk::damage_on(&e, wall), 3);
    assert_eq!(lk::damage_on(&e, angel), 1);
    assert_eq!(lk::zone_of(&e, elves), Zone::Graveyard);
    assert_eq!(lk::life(&e, P1), 20);
}

/// Every chosen target needs a positive share: X=1 permits one target,
/// while X=0 finishes with no target and no life payment. "Any number"
/// also permits choosing no targets with a positive X, still paying X life.
#[test]
fn fire_covenant_handles_zero_targets_and_bounds_target_count_by_x() {
    for x in [0, 1, 4] {
        let mut e = prepared(&[], &[index::WALL_OF_STONE, index::SERRA_ANGEL], &[]);
        let wall = lk::obj(&e, P1, index::WALL_OF_STONE);
        let angel = lk::obj(&e, P1, index::SERRA_ANGEL);
        announce(&mut e, x);
        if x > 0 {
            let Pending::ChooseTargets { min, max, .. } = e.pending() else {
                panic!("targets")
            };
            assert_eq!((*min, *max), (0, x.min(2)));
            if x == 1 {
                assert!(
                    e.apply(
                        P0,
                        PlayerAction::ChooseTargets {
                            objects: vec![wall, angel],
                            players: vec![]
                        }
                    )
                    .is_err()
                );
                choose(&mut e, &[wall]);
            } else {
                choose(&mut e, &[]);
            }
        }
        assert!(
            matches!(e.pending(), Pending::Priority { .. }),
            "no distribution question for zero or one target"
        );
        finish(&mut e);
        assert_eq!(lk::life(&e, P0), 20 - i32::try_from(x).unwrap());
        assert_eq!(lk::damage_on(&e, wall), u16::from(x == 1));
        assert_eq!(lk::damage_on(&e, angel), 0);
        assert!(in_graveyard(&e, P0, fire_covenant()).is_some());
    }
}

/// The life available bounds X, and cancelling after target/share choices
/// returns the card and leaves both life and mana untouched.
#[test]
fn fire_covenant_bounds_x_by_life_and_cancels_before_paying() {
    let mut e = prepared(&[], &[index::WALL_OF_STONE, index::SERRA_ANGEL], &[]);
    let wall = lk::obj(&e, P1, index::WALL_OF_STONE);
    let angel = lk::obj(&e, P1, index::SERRA_ANGEL);
    cast_with_floating(&mut e, P0, fire_covenant());
    assert!(matches!(
        e.pending(),
        Pending::ChooseNumber {
            min: 0,
            max: 20,
            reason: NumberPrompt::X,
            ..
        }
    ));
    assert!(e.apply(P0, PlayerAction::ChooseNumber(21)).is_err());
    e.apply(P0, PlayerAction::ChooseNumber(20)).unwrap();
    choose(&mut e, &[wall, angel]);
    e.apply(P0, PlayerAction::CancelCast).unwrap();
    assert_eq!(lk::life(&e, P0), 20);
    assert_eq!(lk::pool_total(&e, P0), 3);
    assert!(in_hand(&e, P0, fire_covenant()).is_some());
    assert!(stack_is_empty(&e));
}

/// Paying life is a casting cost: Counterspell stops all damage but cannot
/// refund the five life already paid.
#[test]
fn fire_covenant_keeps_life_paid_when_countered() {
    let mut e = prepared(&[], &[index::WALL_OF_STONE], &[index::COUNTERSPELL]);
    let wall = lk::obj(&e, P1, index::WALL_OF_STONE);
    announce(&mut e, 5);
    choose(&mut e, &[wall]);
    assert_eq!(lk::life(&e, P0), 15);
    let covenant = on_stack(&e, fire_covenant()).unwrap();
    priority(&mut e, P1);
    lk::float(&mut e, P1, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut e, P1, index::COUNTERSPELL);
    choose(&mut e, &[covenant]);
    finish(&mut e);
    assert_eq!(lk::life(&e, P0), 15);
    assert_eq!(lk::damage_on(&e, wall), 0);
    assert!(in_graveyard(&e, P0, fire_covenant()).is_some());
}

/// An illegal target loses only its announced share; if every target
/// leaves, no damage is dealt. In either case the life remains paid.
#[test]
fn fire_covenant_does_not_redistribute_an_illegal_targets_damage() {
    for all_illegal in [false, true] {
        let mut e = prepared(
            &[],
            &[index::WALL_OF_STONE, index::SERRA_ANGEL],
            &[index::UNSUMMON],
        );
        let wall = lk::obj(&e, P1, index::WALL_OF_STONE);
        let angel = lk::obj(&e, P1, index::SERRA_ANGEL);
        announce(&mut e, 5);
        if all_illegal {
            choose(&mut e, &[wall]);
        } else {
            choose(&mut e, &[wall, angel]);
            e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
        }
        priority(&mut e, P1);
        lk::float(&mut e, P1, &[(ManaColor::Blue, 1)]);
        cast_with_floating(&mut e, P1, index::UNSUMMON);
        choose(&mut e, &[wall]);
        finish(&mut e);
        assert!(in_hand(&e, P1, index::WALL_OF_STONE).is_some());
        assert_eq!(lk::damage_on(&e, angel), if all_illegal { 0 } else { 2 });
        assert_eq!(lk::life(&e, P0), 15);
        assert!(in_graveyard(&e, P0, fire_covenant()).is_some());
    }
}

/// Damage prevention consumes its target's share and does not reduce the
/// life cost or increase another creature's damage.
#[test]
fn fire_covenant_damage_can_be_prevented_without_refunding_life() {
    let mut e = prepared(
        &[],
        &[index::WALL_OF_STONE, index::SERRA_ANGEL],
        &[index::HEALING_SALVE],
    );
    let wall = lk::obj(&e, P1, index::WALL_OF_STONE);
    let angel = lk::obj(&e, P1, index::SERRA_ANGEL);
    announce(&mut e, 5);
    choose(&mut e, &[wall, angel]);
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    priority(&mut e, P1);
    lk::float(&mut e, P1, &[(ManaColor::White, 1)]);
    cast_with_floating(&mut e, P1, index::HEALING_SALVE);
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        panic!("Salve mode")
    };
    let slot = options
        .iter()
        .position(|option| matches!(option.kind, CastModeKind::Mode(1)))
        .unwrap();
    e.apply(P1, PlayerAction::ChooseMode(slot)).unwrap();
    choose(&mut e, &[wall]);
    finish(&mut e);
    assert_eq!(lk::damage_on(&e, wall), 0);
    assert_eq!(lk::damage_on(&e, angel), 2);
    assert_eq!(lk::life(&e, P0), 15);
}

/// Redirection uses the normal damage pipeline: the Incarnation takes two
/// of its three-point share, its owner takes one, and the other share stays two.
#[test]
fn fire_covenant_damage_can_be_redirected() {
    let mut e = prepared(&[], &[index::PERSONAL_INCARNATION, index::SERRA_ANGEL], &[]);
    let incarnation = lk::obj(&e, P1, index::PERSONAL_INCARNATION);
    let angel = lk::obj(&e, P1, index::SERRA_ANGEL);
    announce(&mut e, 5);
    choose(&mut e, &[incarnation, angel]);
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    priority(&mut e, P1);
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: incarnation,
            ability_index: 0,
        },
    )
    .unwrap();
    finish(&mut e);
    assert_eq!(lk::damage_on(&e, incarnation), 2);
    assert_eq!(lk::damage_on(&e, angel), 2);
    assert_eq!(lk::life(&e, P1), 19);
    assert_eq!(lk::life(&e, P0), 15);
}
