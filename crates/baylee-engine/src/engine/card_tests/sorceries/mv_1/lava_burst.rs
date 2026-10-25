//! `cards/sorceries/mv_1/lava_burst.rs`, played against recipient-specific damage rules.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lava_burst() -> CardIndex {
    card_index("f7be3da5-55b2-46f2-a5aa-277dee242b94")
}

fn prepared(creature: CardIndex, salve: bool) -> Engine<RegistryLookup> {
    let mut e = lk::game(
        SEED,
        &[],
        &[creature],
        &[lava_burst()],
        if salve { &[index::HEALING_SALVE] } else { &[] },
    );
    lk::float(&mut e, P0, &[(ManaColor::Red, 4)]);
    e
}

fn target(e: &mut Engine<RegistryLookup>, objects: &[ObjectId], players: &[PlayerId]) {
    cast_with_floating(e, P0, lava_burst());
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    lk::aim(e, objects, players);
}

fn their_priority(e: &mut Engine<RegistryLookup>) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
}

fn salve(e: &mut Engine<RegistryLookup>, objects: &[ObjectId], players: &[PlayerId]) {
    lk::float(e, P1, &[(ManaColor::White, 1)]);
    cast_with_floating(e, P1, index::HEALING_SALVE);
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        panic!("Salve mode")
    };
    let slot = options
        .iter()
        .position(|option| matches!(option.kind, CastModeKind::Mode(1)))
        .unwrap();
    e.apply(P1, PlayerAction::ChooseMode(slot)).unwrap();
    lk::aim(e, objects, players);
}

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
            pending => panic!("unexpected damage question: {pending:?}"),
        }
    }
    panic!("Lava Burst did not finish");
}

/// The same shield prevents damage to a player, but cannot prevent Lava
/// Burst damage to a creature. Both halves use the card's real damage event.
#[test]
fn lava_burst_damage_is_unpreventable_only_for_creatures() {
    for to_creature in [false, true] {
        let mut e = prepared(index::WALL_OF_STONE, true);
        let wall = lk::obj(&e, P1, index::WALL_OF_STONE);
        let creature_targets = [wall];
        let player_targets = [P1];
        let objects: &[ObjectId] = if to_creature { &creature_targets } else { &[] };
        let players: &[PlayerId] = if to_creature { &[] } else { &player_targets };
        target(&mut e, objects, players);
        their_priority(&mut e);
        salve(&mut e, objects, players);
        finish(&mut e);
        assert_eq!(lk::damage_on(&e, wall), if to_creature { 3 } else { 0 });
        assert_eq!(lk::life(&e, P1), 20);
        assert_eq!(lk::life(&e, P0), 20);
    }
}

/// Personal Incarnation normally redirects its next point to its owner.
/// Lava Burst still deals all three to the creature, and none to its owner.
#[test]
fn lava_burst_damage_to_a_creature_cannot_be_redirected() {
    let mut e = prepared(index::PERSONAL_INCARNATION, false);
    let incarnation = lk::obj(&e, P1, index::PERSONAL_INCARNATION);
    target(&mut e, &[incarnation], &[]);
    their_priority(&mut e);
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: incarnation,
            ability_index: 0,
        },
    )
    .unwrap();
    finish(&mut e);
    assert_eq!(lk::damage_on(&e, incarnation), 3);
    assert_eq!(lk::life(&e, P1), 20);
}

/// The spell deals the chosen X, including zero; the creature-specific
/// rider does not increase X or turn a targeted player into a creature.
#[test]
fn lava_burst_deals_the_announced_x_to_any_target() {
    for x in [0, 3] {
        let mut e = prepared(index::WALL_OF_STONE, false);
        cast_with_floating(&mut e, P0, lava_burst());
        e.apply(P0, PlayerAction::ChooseNumber(x)).unwrap();
        lk::aim(&mut e, &[], &[P1]);
        finish(&mut e);
        assert_eq!(lk::life(&e, P1), 20 - i32::try_from(x).unwrap());
        assert_eq!(lk::pool_total(&e, P0), u64::from(3 - x));
    }
}

/// A player-targeted Burst may be redirected onto a creature by Pariah.
/// Once it would hit that creature, the creature-specific prohibition stops
/// Personal Incarnation redirecting one point back to its owner.
#[test]
fn lava_burst_applies_its_creature_rule_after_redirection_to_a_creature() {
    let mut e = lk::game(
        SEED,
        &[index::PERSONAL_INCARNATION],
        &[],
        &[lava_burst(), index::PARIAH],
        &[],
    );
    let incarnation = lk::obj(&e, P0, index::PERSONAL_INCARNATION);
    lk::float(&mut e, P0, &[(ManaColor::White, 3), (ManaColor::Red, 4)]);
    cast_with_floating(&mut e, P0, index::PARIAH);
    lk::aim(&mut e, &[incarnation], &[]);
    lk::settle(&mut e);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: incarnation,
            ability_index: 0,
        },
    )
    .unwrap();
    lk::settle(&mut e);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    target(&mut e, &[], &[P0]);
    finish(&mut e);
    assert_eq!(lk::damage_on(&e, incarnation), 3);
    assert_eq!(lk::life(&e, P0), 20);
}

/// A planeswalker is an any-target recipient, but is not a creature: its
/// incoming damage remains preventable and its loyalty is preserved.
#[test]
fn lava_burst_damage_to_a_planeswalker_remains_preventable() {
    let mut e = prepared(index::KARN_THE_GREAT_CREATOR, true);
    let walker = lk::obj(&e, P1, index::KARN_THE_GREAT_CREATOR);
    target(&mut e, &[walker], &[]);
    their_priority(&mut e);
    salve(&mut e, &[walker], &[]);
    finish(&mut e);
    assert_eq!(
        e.state()
            .object(walker)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        5
    );
}
