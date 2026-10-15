//! Triassic Egg: actual counter costs, gated modal sacrifice, and both payloads.
// oracle_id = "4f27abe8-3e98-4bb8-afdb-e4b718d83032"

#[allow(clippy::wildcard_imports)] // Shared canonical-card vocabulary.
use super::*;
use crate::engine::card_tests::{legends_kit as lk, partials_w1b_kit as kit};
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn game(hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::TRIASSIC_EGG, index::GRIZZLY_BEARS])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, hand)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn egg(e: &Engine<RegistryLookup>) -> ObjectId {
    on_battlefield(e, P0, index::TRIASSIC_EGG).unwrap()
}

fn choose_mode(e: &mut Engine<RegistryLookup>, mode: usize) {
    let Pending::ChooseCastMode {
        player, options, ..
    } = e.pending()
    else {
        panic!("modal activation: {:?}", e.pending())
    };
    assert_eq!(*player, P0);
    let slot = options
        .iter()
        .position(|o| o.kind == CastModeKind::Mode(mode))
        .expect("legal mode");
    e.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
}

fn hatch_twice(e: &mut Engine<RegistryLookup>) -> ObjectId {
    let source = egg(e);
    assert!(!priority_offer(e).abilities.contains(&(source, 1)));
    for n in 1..=2 {
        lk::float(e, P0, &[(ManaColor::Colorless, 3)]);
        kit::activate_object(e, P0, source, 0);
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
        assert!(is_tapped(e, source));
        pass_until(e, stack_is_empty);
        assert_eq!(counters_on(e, source, counters::HATCHLING), n);
        if n == 1 {
            assert!(!priority_offer(e).abilities.contains(&(source, 1)));
            reach_their_main_phase(e, P1);
            reach_their_main_phase(e, P0);
            assert!(!is_tapped(e, source));
        }
    }
    assert!(priority_offer(e).abilities.contains(&(source, 1)));
    source
}

#[test]
fn batch60_egg_real_hatchling_costs_and_tapped_sacrifice_put_expensive_creature_untapped() {
    let mut e = game(&[index::ROOTBREAKER_WURM, index::FOREST]);
    let source = hatch_twice(&mut e);
    let wurm = in_hand(&e, P0, index::ROOTBREAKER_WURM).unwrap();
    let casts = e.state().per_turn.spells_cast[0];
    kit::activate_object(&mut e, P0, source, 1);
    assert!(
        on_battlefield(&e, P0, index::TRIASSIC_EGG).is_some(),
        "mode precedes sacrifice"
    );
    assert!(is_tapped(&e, source));
    choose_mode(&mut e, 0);
    assert!(in_graveyard(&e, P0, index::TRIASSIC_EGG).is_some());
    assert_eq!(counters_on(&e, source, counters::HATCHLING), 0);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!(*player, P0);
    assert_eq!((*min, *max), (0, 1));
    assert_eq!(options, &vec![wurm]);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![wurm],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::ROOTBREAKER_WURM).is_some());
    assert!(!is_tapped(&e, wurm));
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert_eq!(e.state().per_turn.spells_cast[0], casts);
}

#[test]
fn batch60_egg_optional_hand_mode_can_decline_and_empty_hand_still_has_mode() {
    for hand in [&[index::ROOTBREAKER_WURM][..], &[][..]] {
        let mut e = game(hand);
        let source = hatch_twice(&mut e);
        kit::activate_object(&mut e, P0, source, 1);
        let Pending::ChooseCastMode { options, .. } = e.pending() else {
            panic!("mode")
        };
        assert_eq!(options.len(), 1, "graveyard mode has no creature target");
        assert_eq!(options[0].kind, CastModeKind::Mode(0));
        choose_mode(&mut e, 0);
        if !hand.is_empty() {
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseCards { .. })
            });
            e.apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
                .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert!(in_graveyard(&e, P0, index::TRIASSIC_EGG).is_some());
        assert!(on_battlefield(&e, P0, index::ROOTBREAKER_WURM).is_none());
        if !hand.is_empty() {
            assert!(in_hand(&e, P0, index::ROOTBREAKER_WURM).is_some());
        }
    }
}

fn kill_bear(e: &mut Engine<RegistryLookup>, owner: PlayerId) -> ObjectId {
    let bear = on_battlefield(e, owner, index::GRIZZLY_BEARS).unwrap();
    lk::float(e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(e, P0, index::LIGHTNING_BOLT);
    lk::aim(e, &[bear], &[]);
    pass_until(e, stack_is_empty);
    bear
}

#[test]
fn batch60_egg_graveyard_mode_targets_only_own_creature_before_sacrificing() {
    let mut e = game(&[index::LIGHTNING_BOLT, index::LIGHTNING_BOLT]);
    let own = kill_bear(&mut e, P0);
    let other = kill_bear(&mut e, P1);
    let source = hatch_twice(&mut e);
    kit::activate_object(&mut e, P0, source, 1);
    choose_mode(&mut e, 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = e.pending()
    else {
        panic!("graveyard target")
    };
    assert_eq!(*player, P0);
    assert_eq!(options, &vec![own]);
    assert!(!options.contains(&other));
    assert!(on_battlefield(&e, P0, index::TRIASSIC_EGG).is_some());
    lk::aim(&mut e, &[own], &[]);
    assert!(in_graveyard(&e, P0, index::TRIASSIC_EGG).is_some());
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(!is_tapped(&e, own));
    assert!(in_graveyard(&e, P1, index::GRIZZLY_BEARS).is_some());
}

#[test]
fn batch60_egg_exiled_graveyard_target_fizzles_without_refunding_sacrifice() {
    let mut e = game(&[index::LIGHTNING_BOLT, index::SCAVENGING_OOZE]);
    let own = kill_bear(&mut e, P0);
    lk::float(&mut e, P0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, P0, index::SCAVENGING_OOZE);
    pass_until(&mut e, stack_is_empty);
    let ooze = on_battlefield(&e, P0, index::SCAVENGING_OOZE).unwrap();
    let source = hatch_twice(&mut e);
    kit::activate_object(&mut e, P0, source, 1);
    choose_mode(&mut e, 1);
    lk::aim(&mut e, &[own], &[]);
    lk::float(&mut e, P0, &[(ManaColor::Green, 1)]);
    kit::activate_object(&mut e, P0, ooze, 0);
    lk::aim(&mut e, &[own], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(own).unwrap().zone, Zone::Exile);
    assert_eq!(e.state().object(source).unwrap().zone, Zone::Graveyard);
    assert_eq!(counters_on(&e, ooze, CounterKind::P1P1), 1);
    assert_eq!(e.state().players[0].life, 21);
    assert!(on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_none());
}

#[test]
fn batch60_egg_unrelated_counters_do_not_satisfy_gate_and_more_than_two_do() {
    let mut e = game(&[]);
    let source = egg(&e);
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(source)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 3);
    e.refresh_offer();
    assert!(!priority_offer(&e).abilities.contains(&(source, 1)));
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(source)
        .unwrap()
        .counters
        .set(counters::HATCHLING, 3);
    e.refresh_offer();
    assert!(priority_offer(&e).abilities.contains(&(source, 1)));
    kit::activate_object(&mut e, P0, source, 1);
    choose_mode(&mut e, 0);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, index::TRIASSIC_EGG).is_some());
}

#[test]
fn batch60_egg_putting_creature_is_not_casting_and_its_entry_trigger_resolves() {
    let mut e = game(&[index::ELVISH_VISIONARY]);
    let source = hatch_twice(&mut e);
    let visionary = in_hand(&e, P0, index::ELVISH_VISIONARY).unwrap();
    let before = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    let casts = e.state().per_turn.spells_cast[0];
    kit::activate_object(&mut e, P0, source, 1);
    choose_mode(&mut e, 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![visionary],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::ELVISH_VISIONARY).is_some());
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        before,
        "one card entered and ETB drew one"
    );
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert_eq!(e.state().per_turn.spells_cast[0], casts);
}
