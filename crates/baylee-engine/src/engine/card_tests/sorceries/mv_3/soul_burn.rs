//! `cards/sorceries/mv_3/soul_burn.rs`, played against payment and drain caps.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn soul_burn() -> CardIndex {
    card_index("063b0f5d-af27-4681-87b9-b553f6887061")
}

fn prepared(board: &[CardIndex], mana: &[(ManaColor, u32)]) -> Engine<RegistryLookup> {
    let mut e = lk::game(SEED, &[], board, &[soul_burn()], &[]);
    lk::float(&mut e, P0, mana);
    e
}

fn cast(e: &mut Engine<RegistryLookup>, x: u32, objects: &[ObjectId], players: &[PlayerId]) {
    cast_with_floating(e, P0, soul_burn());
    e.apply(P0, PlayerAction::ChooseNumber(x)).unwrap();
    lk::aim(e, objects, players);
}

/// Red may pay X, but red-paid X gives no life. With {2} paid in colorless,
/// the mixed payment has exactly two black spent on X, excluding the fixed B.
#[test]
fn soul_burn_accepts_red_x_and_caps_life_at_black_spent_specifically_on_x() {
    for (mana, gain) in [
        (
            vec![
                (ManaColor::Colorless, 2),
                (ManaColor::Black, 1),
                (ManaColor::Red, 3),
            ],
            0,
        ),
        (
            vec![
                (ManaColor::Colorless, 2),
                (ManaColor::Black, 3),
                (ManaColor::Red, 1),
            ],
            2,
        ),
        (vec![(ManaColor::Colorless, 2), (ManaColor::Black, 4)], 3),
    ] {
        let mut e = prepared(&[index::WALL_OF_STONE], &mana);
        let wall = lk::obj(&e, P1, index::WALL_OF_STONE);
        cast(&mut e, 3, &[wall], &[]);
        lk::settle(&mut e);
        assert_eq!(lk::damage_on(&e, wall), 3);
        assert_eq!(lk::life(&e, P0), 20 + gain);
        assert_eq!(lk::pool_total(&e, P0), 0);
    }
}

/// X cannot consume blue or green mana even though they can pay the printed
/// generic {2}. The fixed black symbol does not count as black paid on X.
#[test]
fn soul_burn_refuses_nonblack_nonred_mana_for_x() {
    let mut e = prepared(&[], &[(ManaColor::Black, 1), (ManaColor::Blue, 5)]);
    cast_with_floating(&mut e, P0, soul_burn());
    let Pending::ChooseNumber { min, max, .. } = e.pending() else {
        panic!("X")
    };
    assert_eq!((*min, *max), (0, 0));
    assert!(e.apply(P0, PlayerAction::ChooseNumber(1)).is_err());
    e.apply(P0, PlayerAction::ChooseNumber(0)).unwrap();
    lk::aim(&mut e, &[], &[P1]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, P0), 20);
    assert_eq!(lk::life(&e, P1), 20);
    assert_eq!(lk::pool_total(&e, P0), 3);
}

/// Full black payment still obeys the target's toughness and prior loyalty.
/// Damage itself is X; only the resulting life gain has the extra cap.
#[test]
fn soul_burn_caps_life_gain_at_creature_toughness_and_planeswalker_prior_loyalty() {
    for (target_card, gain) in [
        (index::GRIZZLY_BEARS, 2),
        (index::KARN_THE_GREAT_CREATOR, 5),
    ] {
        let mut e = prepared(&[target_card], &[(ManaColor::Black, 9)]);
        let target = lk::obj(&e, P1, target_card);
        cast(&mut e, 6, &[target], &[]);
        lk::settle(&mut e);
        assert_eq!(lk::zone_of(&e, target), Zone::Graveyard);
        assert_eq!(lk::life(&e, P0), 20 + gain);
        assert_eq!(lk::life(&e, P1), 20);
    }
}

/// The player's life total before the damage is another ceiling, even when
/// the damage exceeds it. A zero-X cast gains no life.
#[test]
fn soul_burn_caps_player_drain_at_life_before_damage() {
    for x in [0, 5] {
        let mut e = Duel::new(SEED, forest())
            .life(1, 3)
            .hand(0, &[soul_burn()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        lk::float(&mut e, P0, &[(ManaColor::Black, x + 3)]);
        cast(&mut e, x, &[], &[P1]);
        lk::settle(&mut e);
        assert_eq!(lk::life(&e, P0), if x == 0 { 20 } else { 23 });
        assert_eq!(lk::life(&e, P1), 3 - i32::try_from(x).unwrap());
    }
}

/// A prevention shield can stop the damage; life gain is the damage actually
/// dealt, not the announced X or the black mana receipt alone.
#[test]
fn soul_burn_gains_no_life_for_prevented_damage() {
    let mut e = lk::game(
        SEED,
        &[],
        &[index::WALL_OF_STONE],
        &[soul_burn()],
        &[index::HEALING_SALVE],
    );
    lk::float(&mut e, P0, &[(ManaColor::Black, 6)]);
    let wall = lk::obj(&e, P1, index::WALL_OF_STONE);
    cast(&mut e, 3, &[wall], &[]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
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
    lk::aim(&mut e, &[wall], &[]);
    for _ in 0..40 {
        if stack_is_empty(&e) {
            break;
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
                e.apply(
                    player,
                    PlayerAction::AllocatePrevention {
                        choice,
                        allocation: vec![(damage[0].id, total)],
                    },
                )
                .unwrap();
            }
            pending => panic!("unexpected damage question: {pending:?}"),
        }
    }
    assert!(stack_is_empty(&e));
    assert_eq!(lk::damage_on(&e, wall), 0);
    assert_eq!(lk::life(&e, P0), 20);
}
