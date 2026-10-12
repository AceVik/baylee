//! Actual payments, countered originals and source-independent delayed copies.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::{Cause, GameEvent};

fn back(e: &mut Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    let id = on_battlefield(e, P0, card).unwrap();
    e.state
        .switch_face(id, RegistryLookup.card(card).unwrap(), 1);
    e.sync_static_effects();
    e.refresh_offer();
    id
}

fn copies(e: &Engine<RegistryLookup>) -> usize {
    e.journal()
        .entries()
        .iter()
        .filter(|entry| matches!(entry.event, GameEvent::SpellCopied { .. }))
        .count()
}

fn settle(e: &mut Engine<RegistryLookup>) {
    for _ in 0..100 {
        if stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. }) {
            return;
        }
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player,
                min: 0,
                reason: crate::choice::TargetPrompt::Retarget { .. },
                ..
            } => {
                e.apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![],
                    },
                )
                .unwrap();
            }
            other => panic!("unexpected payment-copy question: {other:?}"),
        }
    }
    panic!("copy stack did not settle");
}

fn wellspring_mana(e: &mut Engine<RegistryLookup>, color: ManaColor) {
    e.refresh_offer();
    activate(e, P0, index::PRIMAL_AMULET, 0);
    e.apply(P0, PlayerAction::ChooseColor(color)).unwrap();
}

#[test]
fn batch46_primal_reduces_only_generic_and_removes_all_counters_on_optional_transform() {
    for accept in [false, true] {
        let mut e = prepared(
            &[index::COUNSEL_OF_THE_SORATAMI],
            &[index::PRIMAL_AMULET],
            &[],
        );
        let amulet = on_battlefield(&e, P0, index::PRIMAL_AMULET).unwrap();
        e.state
            .object_mut(amulet)
            .unwrap()
            .counters
            .add(CounterKind::Charge, 6);
        reset_pool(&mut e);
        e.state.players[0].mana_pool.add(ManaColor::Colorless, 2);
        let spell = in_hand(&e, P0, index::COUNSEL_OF_THE_SORATAMI).unwrap();
        assert!(
            crate::casting::can_cast(e.state(), &RegistryLookup, P0, spell).is_err(),
            "blue is still required"
        );
        e.state.players[0].mana_pool = ManaPool::new();
        e.state.players[0].mana_pool.add(ManaColor::Colorless, 1);
        e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
        cast(&mut e, spell);
        assert_eq!(e.state.players[0].mana_pool.total(), 0);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        assert_eq!(
            e.state
                .object(amulet)
                .unwrap()
                .counters
                .get(CounterKind::Charge),
            7
        );
        yes(&mut e, accept);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.object(amulet).unwrap().face_index, u8::from(accept));
        assert_eq!(
            e.state
                .object(amulet)
                .unwrap()
                .counters
                .get(CounterKind::Charge),
            if accept { 0 } else { 7 }
        );
    }
}

#[test]
fn batch46_wellspring_copies_each_spent_unit_after_source_and_original_leave() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::COUNSEL_OF_THE_SORATAMI])
        .hand(1, &[index::COUNTERSPELL])
        .battlefield(0, &[index::PRIMAL_AMULET])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let well = back(&mut e, index::PRIMAL_AMULET);
    wellspring_mana(&mut e, ManaColor::Blue);
    e.state.set_tapped(well, false);
    wellspring_mana(&mut e, ManaColor::Blue);
    assert_eq!(e.state.players[0].mana_pool.total(), 2);
    e.state
        .move_object(
            well,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 1);
    e.state.players[1].mana_pool.add(ManaColor::Blue, 2);
    let spell = in_hand(&e, P0, index::COUNSEL_OF_THE_SORATAMI).unwrap();
    let before = e.state.zones.list(ZoneLocation::Hand(P0)).len();
    cast(&mut e, spell);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    let counter = in_hand(&e, P1, index::COUNTERSPELL).unwrap();
    e.apply(P1, PlayerAction::CastSpell { card: counter })
        .unwrap();
    target(&mut e, vec![spell], vec![]);
    settle(&mut e);
    assert_eq!(
        copies(&e),
        2,
        "one copy per paid unit, even without the source or original spell"
    );
    assert_eq!(
        e.state.zones.list(ZoneLocation::Hand(P0)).len(),
        before - 1 + 4
    );
}

#[test]
fn batch46_wellspring_mana_can_pay_nonmatching_spells_without_copying_them() {
    let mut e = prepared(&[index::SOL_RING], &[index::PRIMAL_AMULET], &[]);
    back(&mut e, index::PRIMAL_AMULET);
    reset_pool(&mut e);
    wellspring_mana(&mut e, ManaColor::Blue);
    let spell = in_hand(&e, P0, index::SOL_RING).unwrap();
    cast(&mut e, spell);
    settle(&mut e);
    assert_eq!(copies(&e), 0);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    assert!(on_battlefield(&e, P0, index::SOL_RING).is_some());
}

fn palace_mana(e: &mut Engine<RegistryLookup>) {
    let palace = on_battlefield(e, P0, index::SUNKEN_PALACE).unwrap();
    e.state.set_tapped(palace, false);
    seed_graveyard(e, P0, 7);
    e.refresh_offer();
    activate(e, P0, index::SUNKEN_PALACE, 1);
    // The seven exile parts select distinct cards, collecting all costs before payment.
    for _ in 0..7 {
        let Pending::ChooseCards {
            player,
            options,
            min,
            ..
        } = e.pending().clone()
        else {
            break;
        };
        let n = usize::from(min);
        assert!(n > 0);
        e.apply(
            player,
            PlayerAction::ChooseObjects {
                objects: options[..n].to_vec(),
            },
        )
        .unwrap();
    }
    assert!(matches!(e.pending(), Pending::Priority { .. }));
    assert!(e.state.zones.list(ZoneLocation::Graveyard(P0)).is_empty());
    assert!(
        stack_is_empty(e),
        "the Palace's production is a mana ability"
    );
}

#[test]
fn batch46_palace_copies_an_activated_ability_but_not_a_mana_ability() {
    for mana_ability in [false, true] {
        let mut e = prepared(
            &[],
            &[
                index::SUNKEN_PALACE,
                index::JAYEMDAE_TOME,
                index::APPRENTICE_WIZARD,
            ],
            &[],
        );
        reset_pool(&mut e);
        e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
        e.state.players[0].mana_pool.add(ManaColor::Colorless, 1);
        palace_mana(&mut e);
        assert_eq!(e.state.players[0].mana_pool.total(), 1);
        let before = e.state.zones.list(ZoneLocation::Hand(P0)).len();
        if mana_ability {
            activate(&mut e, P0, index::APPRENTICE_WIZARD, 0);
            assert!(stack_is_empty(&e));
            assert_eq!(e.state.players[0].mana_pool.total(), 3);
        } else {
            e.state.players[0].mana_pool.add(ManaColor::Colorless, 3);
            e.refresh_offer();
            activate(&mut e, P0, index::JAYEMDAE_TOME, 0);
            settle(&mut e);
            assert_eq!(e.state.zones.list(ZoneLocation::Hand(P0)).len(), before + 2);
        }
    }
}

fn settle_retargeting(
    e: &mut Engine<RegistryLookup>,
    objects: &[ObjectId],
    players: &[PlayerId],
) -> usize {
    let mut prompts = 0;
    for _ in 0..100 {
        if stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. }) {
            return prompts;
        }
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player,
                min: 0,
                reason: crate::choice::TargetPrompt::Retarget { .. },
                ..
            } => {
                prompts += 1;
                e.apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: objects.to_vec(),
                        players: players.to_vec(),
                    },
                )
                .unwrap();
            }
            other => panic!("unexpected copy question: {other:?}"),
        }
    }
    panic!("copy did not settle");
}

#[test]
fn batch46_wellspring_copy_may_change_or_keep_the_original_target() {
    for change_target in [false, true] {
        let mut e = prepared(&[index::LIGHTNING_BOLT], &[index::PRIMAL_AMULET], &[]);
        back(&mut e, index::PRIMAL_AMULET);
        reset_pool(&mut e);
        wellspring_mana(&mut e, ManaColor::Red);
        let spell = in_hand(&e, P0, index::LIGHTNING_BOLT).unwrap();
        let life = [e.state.players[0].life, e.state.players[1].life];
        cast(&mut e, spell);
        target(&mut e, vec![], vec![P1]);
        let new_targets = if change_target { vec![P0] } else { vec![] };
        assert_eq!(settle_retargeting(&mut e, &[], &new_targets), 1);
        assert_eq!(
            e.state.players[0].life,
            life[0] - if change_target { 3 } else { 0 }
        );
        assert_eq!(
            e.state.players[1].life,
            life[1] - if change_target { 3 } else { 6 }
        );
    }
}

#[test]
fn batch46_palace_copy_preserves_announced_activation_x_and_may_retarget() {
    let mut e = prepared(
        &[],
        &[
            index::SUNKEN_PALACE,
            index::KESSIG_WOLF_RUN,
            index::GRIZZLY_BEARS,
        ],
        &[index::GRIZZLY_BEARS],
    );
    let own_bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let their_bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    reset_pool(&mut e);
    e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 1);
    palace_mana(&mut e);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 1);
    e.state.players[0].mana_pool.add(ManaColor::Red, 1);
    e.state.players[0].mana_pool.add(ManaColor::Green, 1);
    e.refresh_offer();
    activate(&mut e, P0, index::KESSIG_WOLF_RUN, 1);
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    target(&mut e, vec![own_bear], vec![]);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    assert_eq!(settle_retargeting(&mut e, &[their_bear], &[]), 1);
    assert_eq!(pt(&e, own_bear), (4, 2));
    assert_eq!(pt(&e, their_bear), (4, 2));
}
