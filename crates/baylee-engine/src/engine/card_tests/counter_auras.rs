//! Aura counters remember their host, including after the Aura sacrifices itself.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

fn enchant(card: CardIndex, x: u32) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::GRIZZLY_BEARS,
                forest(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let host = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let aura = in_hand(&e, p0, card).unwrap();
    tap_all_mana(&mut e, p0);
    e.apply(p0, PlayerAction::CastSpell { card: aura }).unwrap();
    for _ in 0..8 {
        match e.pending().clone() {
            Pending::ChooseNumber { .. } => e.apply(p0, PlayerAction::ChooseNumber(x)).unwrap(),
            Pending::ChooseTargets { .. } => e
                .apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![host],
                    },
                )
                .unwrap(),
            Pending::Priority { .. } => break,
            other => panic!("unexpected casting choice {other:?}"),
        }
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(aura).unwrap().attached_to, Some(host));
    (e, aura, host)
}

fn next_main(e: &mut Engine<RegistryLookup>) {
    let after = e.state().turn.number;
    pass_until(e, |e| {
        e.state().turn.number > after
            && e.state().turn.active == PlayerId::new(0)
            && e.state().turn.phase == Phase::FirstMain
    });
}

#[test]
fn venarian_gold_counts_announced_x_and_removes_sleep_after_each_missed_untap() {
    let (mut e, aura, host) = enchant(index::VENARIAN_GOLD, 2);
    assert!(is_tapped(&e, host));
    assert_eq!(counters_on(&e, host, counters::SLEEP), 2);
    assert_eq!(counters_on(&e, aura, counters::SLEEP), 0);
    for remaining in [1, 0] {
        next_main(&mut e);
        assert!(is_tapped(&e, host));
        assert_eq!(counters_on(&e, host, counters::SLEEP), remaining);
    }
    next_main(&mut e);
    assert!(!is_tapped(&e, host));
}

#[test]
fn cocoon_hatches_only_on_the_upkeep_after_the_last_pupa_counter() {
    let (mut e, aura, host) = enchant(index::COCOON, 0);
    assert_eq!(counters_on(&e, aura, counters::PUPA), 3);
    for remaining in [2, 1, 0] {
        next_main(&mut e);
        assert!(is_tapped(&e, host));
        assert_eq!(counters_on(&e, aura, counters::PUPA), remaining);
        assert_eq!(pt(&e, host), (2, 2));
        assert!(!keywords(&e, host).contains(KeywordSet::FLYING));
    }
    next_main(&mut e);
    assert!(!is_tapped(&e, host));
    assert_eq!(e.state().object(aura).unwrap().zone, Zone::Graveyard);
    assert_eq!(pt(&e, host), (3, 3));
    assert_eq!(counters_on(&e, host, CounterKind::P1P1), 1);
    assert!(keywords(&e, host).contains(KeywordSet::FLYING));
}

#[test]
fn destroying_venarian_gold_keeps_sleep_counters_but_allows_untapping() {
    let (mut e, aura, host) = enchant(index::VENARIAN_GOLD, 2);
    kill(&mut e, aura);
    assert_eq!(counters_on(&e, host, counters::SLEEP), 2);
    next_main(&mut e);
    assert!(!is_tapped(&e, host));
    assert_eq!(counters_on(&e, host, counters::SLEEP), 2);
}
