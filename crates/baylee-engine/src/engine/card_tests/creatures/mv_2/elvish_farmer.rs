//! `cards/creatures/mv_2/elvish_farmer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn ready(mine: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut all = vec![card_index("91fe1075-e170-4575-ae28-8d692f051637")];
    all.extend_from_slice(mine);
    let mut e = Duel::new(SEED, forest()).battlefield(0, &all).start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn farmer(e: &Engine<RegistryLookup>) -> ObjectId {
    on_battlefield(e, P0, card_index("91fe1075-e170-4575-ae28-8d692f051637"))
        .expect("the Farmer is out")
}

fn spores(e: &Engine<RegistryLookup>) -> u16 {
    e.state()
        .object(farmer(e))
        .expect("the Farmer is out")
        .counters
        .get(baylee_cards_dsl::counters::SPORE)
}

fn next_own_upkeep(e: &mut Engine<RegistryLookup>) {
    pass_until(e, |e| e.state().turn.active == P1);
    pass_until(e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(e, stack_is_empty);
}

fn offers(e: &Engine<RegistryLookup>, index: u32) -> bool {
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected priority, got {:?}", e.pending())
    };
    legal.abilities.contains(&(farmer(e), index))
}

fn saprolings(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| !o.is_card() && o.controller == P0)
        })
        .collect()
}

/// Grows spore counters to three and cashes them in for a Saproling.
fn with_a_saproling(mine: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut e = ready(mine);
    while spores(&e) < 3 {
        next_own_upkeep(&mut e);
    }
    let f = farmer(&e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: f,
            ability_index: 1,
        },
    )
    .expect("three counters pay");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(saprolings(&e).len(), 1, "one Saproling");
    e
}

/// Elvish Farmer (91fe1075-e170-4575-ae28-8d692f051637) — "At the beginning of
/// your upkeep, put a spore counter on this creature. Remove three spore
/// counters from this creature: Create a 1/1 green Saproling creature token.
/// Sacrifice a Saproling: You gain 2 life."
#[test]
fn elvish_farmer_gains_a_spore_counter_on_your_upkeep_only() {
    let mut e = ready(&[]);
    let start = spores(&e);
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut e, stack_is_empty);
    assert_eq!(spores(&e), start, "not on the opponent's upkeep");
    next_own_upkeep(&mut e);
    assert_eq!(spores(&e), start + 1);
}

#[test]
fn elvish_farmer_makes_a_saproling_from_three_spore_counters() {
    let mut e = ready(&[]);
    while spores(&e) < 2 {
        next_own_upkeep(&mut e);
    }
    assert!(!offers(&e, 1), "two counters are not enough");
    next_own_upkeep(&mut e);
    assert_eq!(spores(&e), 3);
    assert!(offers(&e, 1));

    let f = farmer(&e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: f,
            ability_index: 1,
        },
    )
    .expect("three counters pay");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(spores(&e), 0);
    let tokens = saprolings(&e);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&e, tokens[0]), (1, 1));
}

#[test]
fn elvish_farmer_sacrificing_a_saproling_gains_two_life() {
    let mut e = with_a_saproling(&[grizzly_bears()]);
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("the Bears are out");
    let sap = saprolings(&e)[0];
    let life = e.state().players[0].life;
    assert!(offers(&e, 2));

    let f = farmer(&e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: f,
            ability_index: 2,
        },
    )
    .expect("a Saproling is there to sacrifice");
    // A lone choice may be made for the player; if it is asked, the menu
    // is the Saprolings and only they.
    let menu = match e.pending() {
        Pending::ChooseCards { options, .. } => Some(options.clone()),
        _ => None,
    };
    assert!(
        menu.as_ref()
            .is_none_or(|o| o.contains(&sap) && !o.contains(&bears)),
        "the menu is the Saproling alone: {menu:?}"
    );
    if menu.is_some() {
        e.apply(P0, PlayerAction::ChooseObjects { objects: vec![sap] })
            .expect("the Saproling is chosen");
    }
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().players[0].life, life + 2);
    assert!(saprolings(&e).is_empty(), "the Saproling was sacrificed");
    assert!(
        on_battlefield(&e, P0, grizzly_bears()).is_some(),
        "the Bears were not"
    );
    assert!(!offers(&e, 2), "nothing left to sacrifice");
}

#[test]
fn elvish_farmer_cannot_sacrifice_anything_that_is_not_a_saproling() {
    let e = ready(&[grizzly_bears()]);
    assert!(!offers(&e, 2), "Bears are not Saprolings");
}
