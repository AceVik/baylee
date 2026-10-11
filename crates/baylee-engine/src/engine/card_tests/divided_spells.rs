//! Divided damage is announced, paid, copied and resolved without redistribution.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::{NumberPrompt, TargetRef};
use crate::event::Cause;
use crate::zone::ZonePosition;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn setup(card: CardIndex) -> (Engine<RegistryLookup>, [ObjectId; 2]) {
    let mut e = Duel::new(SEED, mountain())
        .hand(0, &[card])
        .battlefield(1, &[index::WALL_OF_STONE, index::DARKSTEEL_GARGOYLE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 30);
    e.refresh_offer();
    let targets = [
        on_battlefield(&e, P1, index::WALL_OF_STONE).unwrap(),
        on_battlefield(&e, P1, index::DARKSTEEL_GARGOYLE).unwrap(),
    ];
    (e, targets)
}

fn cast_front(e: &mut Engine<RegistryLookup>, card: CardIndex) {
    cast_with_floating(e, P0, card);
    if let Pending::ChooseCastMode { options, .. } = e.pending() {
        let index = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Normal | CastModeKind::Face(0)))
            .unwrap();
        e.apply(P0, PlayerAction::ChooseMode(index)).unwrap();
    }
}

fn choose(e: &mut Engine<RegistryLookup>, objects: &[ObjectId], players: &[PlayerId]) {
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: objects.to_vec(),
            players: players.to_vec(),
        },
    )
    .unwrap();
}

fn split(e: &mut Engine<RegistryLookup>, shares: &[u32]) {
    for n in shares.iter().take(shares.len().saturating_sub(1)) {
        assert!(matches!(
            e.pending(),
            Pending::ChooseNumber {
                reason: NumberPrompt::DivideDamage { .. },
                ..
            }
        ));
        e.apply(P0, PlayerAction::ChooseNumber(*n)).unwrap();
    }
    assert!(matches!(e.pending(), Pending::Priority { .. }));
}

#[test]
fn pyrotechnics_all_positive_compositions_of_four_include_objects_and_players() {
    // Every ordered positive composition, across one through four recipients.
    for cuts in 0_u8..8 {
        let mut shares = Vec::new();
        let mut start = 0;
        for position in 1..4 {
            if cuts & (1 << (position - 1)) != 0 {
                shares.push(position - start);
                start = position;
            }
        }
        shares.push(4 - start);
        let (mut e, targets) = setup(index::PYROTECHNICS);
        cast_front(&mut e, index::PYROTECHNICS);
        let players = [P0, P1];
        choose(
            &mut e,
            &targets[..shares.len().min(2)],
            &players[..shares.len().saturating_sub(2)],
        );
        split(&mut e, &shares);
        pass_until(&mut e, stack_is_empty);
        for (i, target) in targets.iter().enumerate() {
            assert_eq!(
                u32::from(e.state().object(*target).unwrap().damage),
                shares.get(i).copied().unwrap_or(0)
            );
        }
        for (i, player) in e.state().players.iter().enumerate() {
            assert_eq!(
                player.life,
                20 - i32::try_from(shares.get(i + 2).copied().unwrap_or(0)).unwrap()
            );
        }
    }
}

#[test]
fn fire_allows_one_player_or_a_split_across_player_and_creature() {
    for with_creature in [false, true] {
        let (mut e, targets) = setup(index::FIRE);
        cast_front(&mut e, index::FIRE);
        choose(
            &mut e,
            if with_creature { &targets[..1] } else { &[] },
            &[P1],
        );
        if with_creature {
            split(&mut e, &[1, 1]);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().players[1].life,
            if with_creature { 19 } else { 18 }
        );
        assert_eq!(
            e.state().object(targets[0]).unwrap().damage,
            u16::from(with_creature)
        );
    }
}

#[test]
fn shatterskull_bounds_targets_by_x_and_divides_the_doubled_total() {
    for x in [0, 1, 5, 6, 7] {
        let (mut e, targets) = setup(index::SHATTERSKULL_SMASHING);
        cast_front(&mut e, index::SHATTERSKULL_SMASHING);
        e.apply(P0, PlayerAction::ChooseNumber(x)).unwrap();
        let count = usize::try_from(x.min(2)).unwrap();
        if count > 0 {
            let Pending::ChooseTargets {
                max,
                player_options,
                ..
            } = e.pending()
            else {
                panic!("{:?}", e.pending());
            };
            assert_eq!(*max, x.min(2));
            assert!(player_options.is_empty());
            choose(&mut e, &targets[..count], &[]);
        }
        let total = if x >= 6 { 2 * x } else { x };
        if count == 2 {
            split(&mut e, &[1, total - 1]);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            u32::from(e.state().object(targets[0]).unwrap().damage),
            u32::from(count > 0)
        );
        assert_eq!(
            u32::from(e.state().object(targets[1]).unwrap().damage),
            if count == 2 { total - 1 } else { 0 }
        );
        assert_eq!(
            e.state().players[0].mana_pool.available(ManaColor::Red),
            28 - x
        );
    }
}

#[test]
fn zero_targets_do_not_ask_an_unanswerable_division() {
    for card in [index::PYROTECHNICS, index::SHATTERSKULL_SMASHING] {
        let (mut e, _) = setup(card);
        cast_front(&mut e, card);
        if card == index::SHATTERSKULL_SMASHING {
            e.apply(P0, PlayerAction::ChooseNumber(6)).unwrap();
        }
        choose(&mut e, &[], &[]);
        assert!(matches!(e.pending(), Pending::Priority { .. }));
        pass_until(&mut e, stack_is_empty);
        assert!(e.state().divided.is_empty());
    }
}

#[test]
fn division_precedes_payment_and_cancellation_restores_the_announced_cast() {
    let (mut e, targets) = setup(index::PYROTECHNICS);
    let before = e.state().loop_signature();
    cast_front(&mut e, index::PYROTECHNICS);
    choose(&mut e, &targets, &[]);
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Red), 30);
    assert!(e.state().divided.is_empty());
    assert!(e.apply(P0, PlayerAction::ChooseNumber(0)).is_err());
    assert!(e.apply(P0, PlayerAction::ChooseNumber(4)).is_err());
    e.apply(P0, PlayerAction::CancelCast).unwrap();
    // The journal deliberately records CastCancelled; compare game meaning.
    assert_eq!(e.state().loop_signature(), before);
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Red), 30);
    assert!(e.state().divided.is_empty());
    assert!(stack_is_empty(&e));
    assert!(matches!(e.pending(), Pending::Priority { player: P0, .. }));
    assert!(in_hand(&e, P0, index::PYROTECHNICS).is_some());
}

#[test]
fn a_blinked_target_loses_its_share_without_increasing_the_players_damage() {
    let (mut e, targets) = setup(index::PYROTECHNICS);
    cast_front(&mut e, index::PYROTECHNICS);
    choose(&mut e, &targets[..1], &[P1]);
    split(&mut e, &[3, 1]);
    let state = e.dev_state_mut(P0).unwrap();
    state
        .move_object(
            targets[0],
            ZoneLocation::Exile(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            targets[0],
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(targets[0]).unwrap().damage, 0);
    assert_eq!(e.state().players[1].life, 19);
}

#[test]
fn player_only_division_uses_player_identities_and_keeps_announced_order() {
    let (mut e, _) = setup(index::PYROTECHNICS);
    cast_front(&mut e, index::PYROTECHNICS);
    choose(&mut e, &[], &[P1, P0]);
    assert!(matches!(
        e.pending(),
        Pending::ChooseNumber {
            reason: NumberPrompt::DivideDamage {
                target: TargetRef::Player(P0),
                ..
            },
            ..
        }
    ));
    split(&mut e, &[1, 3]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(e.state().players[1].life, 17);
}

#[test]
fn pending_distributions_are_part_of_the_replay_fingerprint() {
    let fingerprint = |first_share| {
        let (mut e, targets) = setup(index::PYROTECHNICS);
        cast_front(&mut e, index::PYROTECHNICS);
        choose(&mut e, &targets, &[P1]);
        e.apply(P0, PlayerAction::ChooseNumber(first_share))
            .unwrap();
        assert!(matches!(e.pending(), Pending::ChooseNumber { .. }));
        e.snapshot_hash()
    };
    assert_ne!(fingerprint(1), fingerprint(2));
}
