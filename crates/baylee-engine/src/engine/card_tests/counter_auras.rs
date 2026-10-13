//! Aura counters remember their host, including after the Aura sacrifices itself.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

fn enchant(card: CardIndex, x: u32) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    enchant_creature_of(card, x, 0)
}

/// As [`enchant`], but the Grizzly Bears it enchants is `host_seat`'s: P0
/// casts the Aura either way, so seat 1 is an opponent's creature.
fn enchant_creature_of(
    card: CardIndex,
    x: u32,
    host_seat: usize,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let host_owner = PlayerId::new(u8::try_from(host_seat).unwrap());
    let mut mine = vec![forest(), island(), island(), island(), island()];
    let mut duel = Duel::new(SEED, forest());
    if host_seat == 0 {
        mine.insert(0, index::GRIZZLY_BEARS);
    } else {
        duel = duel.battlefield(host_seat, &[index::GRIZZLY_BEARS]);
    }
    let mut e = duel.battlefield(0, &mine).hand(0, &[card]).start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let host = on_battlefield(&e, host_owner, index::GRIZZLY_BEARS).unwrap();
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

/// "At the beginning of the upkeep of enchanted creature's controller": the
/// sleep counter comes off at the upkeep of whoever controls the host, not
/// the Aura's controller. P0 enchants P1's creature, so P0's upkeeps take
/// nothing off it and P1's take one each; the host stays tapped while a
/// counter remains and is let go only once the last one is gone.
#[test]
fn venarian_gold_on_an_opposing_creature_counts_down_on_that_players_upkeeps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut e, _aura, host) = enchant_creature_of(index::VENARIAN_GOLD, 2, 1);
    assert!(is_tapped(&e, host), "tapped as the Aura enters");
    assert_eq!(counters_on(&e, host, counters::SLEEP), 2);

    // P1's first turn: it does not untap (a counter is on it), and its own
    // upkeep takes one off.
    reach_their_main_phase(&mut e, p1);
    assert!(is_tapped(&e, host), "no untap with a sleep counter on it");
    assert_eq!(
        counters_on(&e, host, counters::SLEEP),
        1,
        "the upkeep of the enchanted creature's controller (P1) removes one"
    );

    // P0's turn: the Aura's controller's upkeep is not the one that counts.
    reach_their_main_phase(&mut e, p0);
    assert_eq!(
        counters_on(&e, host, counters::SLEEP),
        1,
        "the Aura's controller's upkeep removes nothing"
    );
    assert!(is_tapped(&e, host));

    // P1's second turn: still no untap (one counter was left at the untap
    // step), then the last counter comes off at the upkeep.
    reach_their_main_phase(&mut e, p1);
    assert_eq!(counters_on(&e, host, counters::SLEEP), 0);
    assert!(
        is_tapped(&e, host),
        "it did not untap while the counter was still on it"
    );

    reach_their_main_phase(&mut e, p0);
    assert_eq!(counters_on(&e, host, counters::SLEEP), 0);
    reach_their_main_phase(&mut e, p1);
    assert!(!is_tapped(&e, host), "with no counter left it untaps");
}

/// "Enchant creature you control": the menu offered when Cocoon is cast has
/// the caster's creature on it and the opponent's off it.
#[test]
fn cocoon_offers_only_creatures_its_caster_controls() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::GRIZZLY_BEARS, forest()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[index::COCOON])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let mine = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let theirs = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut e, p0, index::COCOON);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected the Aura's target choice, got {:?}", e.pending())
    };
    assert!(options.contains(&mine), "your own creature is enchantable");
    assert!(
        !options.contains(&theirs),
        "\"Enchant creature you control\": an opponent's creature is not offered"
    );
}
