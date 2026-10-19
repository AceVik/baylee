//! `cards/creatures/mv_6/ebon_praetor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);

/// A duel at P0's first main phase with the named hand and boards, and every
/// colour of mana floating for both seats.
fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    e
}

fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p),
    );
}

fn activate_by_card(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex, ability_index: u32) {
    priority(e, p);
    mana(e);
    let source = on_battlefield(e, p, c).expect("the source is on the battlefield");
    e.apply(
        p,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .expect("the ability is activated");
}

fn choose_objects(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(p, PlayerAction::ChooseObjects { objects })
        .expect("the choice is on the menu");
}

fn ebon_praetor() -> CardIndex {
    card_index("0501af1f-c67c-4cb1-aced-4be426c83316")
}

fn basal_thrull() -> CardIndex {
    card_index("9da50130-3f83-4968-983c-7dcba257cf1b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// The Praetor, a Thrull and a Bear on P0's side, walked to P0's second
/// upkeep (turn 3) with the Praetor's trigger resolved: two -2/-2 counters.
fn at_the_second_upkeep() -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = prepared(&[], &[ebon_praetor(), basal_thrull(), grizzly_bears()], &[]);
    let praetor = on_battlefield(&e, P0, ebon_praetor()).expect("the Praetor is out");
    pass_until(&mut e, |e| {
        e.state().turn.number == 3
            && e.state().turn.step == Step::Upkeep
            && pt(e, praetor) == (1, 1)
    });
    (e, praetor)
}

/// Ebon Praetor (0501af1f-c67c-4cb1-aced-4be426c83316) — "First strike, trample"
#[test]
fn ebon_praetor_has_first_strike_and_trample() {
    let e = prepared(&[], &[ebon_praetor()], &[]);
    let praetor = on_battlefield(&e, P0, ebon_praetor()).expect("the Praetor is out");
    assert!(keywords(&e, praetor).contains(KeywordSet::FIRST_STRIKE));
    assert!(keywords(&e, praetor).contains(KeywordSet::TRAMPLE));
}

/// Ebon Praetor — "At the beginning of your upkeep, put a -2/-2 counter on
/// this creature."
#[test]
fn ebon_praetor_shrinks_by_two_at_each_of_its_controllers_upkeeps_only() {
    let mut e = prepared(&[], &[ebon_praetor()], &[]);
    let praetor = on_battlefield(&e, P0, ebon_praetor()).expect("the Praetor is out");
    // Turn 1's upkeep has already passed: 5/5 with one counter.
    assert_eq!(pt(&e, praetor), (3, 3));

    // The opponent's upkeep puts nothing on it.
    pass_until(&mut e, |e| {
        e.state().turn.number == 2 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&e, praetor), (3, 3), "not the opponent's upkeep");

    // P0's next upkeep puts the second.
    pass_until(&mut e, |e| {
        e.state().turn.number == 3
            && e.state().turn.step == Step::Upkeep
            && pt(e, praetor) == (1, 1)
    });
    assert_eq!(
        e.state()
            .object(praetor)
            .unwrap()
            .counters
            .get(CounterKind::Minus {
                power: 2,
                toughness: 2
            }),
        2
    );
}

/// Ebon Praetor — "Sacrifice a creature: Remove a -2/-2 counter from this
/// creature. If the sacrificed creature was a Thrull, put a +1/+0 counter on
/// this creature." (a Thrull)
#[test]
fn ebon_praetor_sacrificing_a_thrull_removes_a_counter_and_adds_a_power_counter() {
    let (mut e, praetor) = at_the_second_upkeep();
    let thrull = on_battlefield(&e, P0, basal_thrull()).expect("Basal Thrull is out");

    activate_by_card(&mut e, P0, ebon_praetor(), 1);
    choose_objects(&mut e, vec![thrull]);
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, P0, basal_thrull()).is_some(), "sacrificed");
    // 5/5, one -2/-2 counter left, one +1/+0 counter.
    assert_eq!(pt(&e, praetor), (4, 3));
}

/// Ebon Praetor — the same sacrifice of a creature that is not a Thrull.
#[test]
fn ebon_praetor_sacrificing_a_non_thrull_only_removes_a_counter() {
    let (mut e, praetor) = at_the_second_upkeep();
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");

    activate_by_card(&mut e, P0, ebon_praetor(), 1);
    choose_objects(&mut e, vec![bears]);
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "sacrificed"
    );
    // 5/5 with one -2/-2 counter left and no power counter.
    assert_eq!(pt(&e, praetor), (3, 3));
}

/// Ebon Praetor — "Activate only during your upkeep and only once each turn."
#[test]
fn ebon_praetor_activates_only_in_its_controllers_upkeep_and_once_a_turn() {
    // Main phase: not offered.
    let e = prepared(&[], &[ebon_praetor(), basal_thrull()], &[]);
    let praetor = on_battlefield(&e, P0, ebon_praetor()).expect("the Praetor is out");
    assert!(
        !priority_offer(&e).abilities.contains(&(praetor, 1)),
        "not in a main phase"
    );

    // Upkeep: offered once, then not again that turn.
    let (mut e, praetor) = at_the_second_upkeep();
    assert!(
        priority_offer(&e).abilities.contains(&(praetor, 1)),
        "offered in P0's upkeep"
    );
    let thrull = on_battlefield(&e, P0, basal_thrull()).expect("Basal Thrull is out");
    activate_by_card(&mut e, P0, ebon_praetor(), 1);
    choose_objects(&mut e, vec![thrull]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, grizzly_bears()).is_some(),
        "another creature is there to sacrifice"
    );
    assert!(
        !priority_offer(&e).abilities.contains(&(praetor, 1)),
        "not a second time this turn"
    );
}
