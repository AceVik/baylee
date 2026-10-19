//! `cards/creatures/artifacts/mv_5/necropolis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ChoicePrompt;

const P0: PlayerId = PlayerId::new(0);

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn table(mine: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest()).battlefield(0, mine).start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

/// Whether the offer lists ability `index` of `card`.
fn offers(e: &Engine<RegistryLookup>, card: CardIndex, index: u32) -> bool {
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected priority, got {:?}", e.pending())
    };
    legal.abilities.iter().any(|(id, ai)| {
        *ai == index
            && e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
    })
}

/// Sends a battlefield permanent to its owner's graveyard.
fn to_graveyard(e: &mut Engine<RegistryLookup>, card: CardIndex) {
    let id = on_battlefield(e, P0, card).expect("the permanent is out");
    bury(e, &[id]);
    e.refresh_offer();
}

/// Activates the exile ability, exiling the graveyard card printed `card`.
fn exile_for_counters(e: &mut Engine<RegistryLookup>, card: CardIndex) {
    let wanted = in_graveyard(e, P0, card).expect("the card is in the graveyard");
    activate(e, P0, card_index("c84adca4-19dd-432d-a363-e98d3b699af8"), 0);
    let Pending::ChooseCards {
        prompt: ChoicePrompt::CostExile,
        options,
        ..
    } = e.pending().clone()
    else {
        panic!("expected the exile cost, got {:?}", e.pending())
    };
    assert!(options.contains(&wanted), "the creature card is on offer");
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![wanted],
        },
    )
    .expect("the exile is paid");
    pass_until(e, stack_is_empty);
}

/// Necropolis (c84adca4-19dd-432d-a363-e98d3b699af8) — "Defender. Exile a
/// creature card from your graveyard: Put X +0/+1 counters on this
/// creature, where X is the exiled card's mana value."
#[test]
fn necropolis_has_defender_and_cannot_be_declared_an_attacker() {
    let mut e = table(&[
        card_index("c84adca4-19dd-432d-a363-e98d3b699af8"),
        grizzly_bears(),
    ]);
    let necro = on_battlefield(&e, P0, card_index("c84adca4-19dd-432d-a363-e98d3b699af8"))
        .expect("Necropolis is out");
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Bears are out");
    assert_eq!(pt(&e, necro), (0, 1));
    assert!(keywords(&e, necro).contains(KeywordSet::DEFENDER));

    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::DeclareAttackers
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        panic!("expected the attack question, got {:?}", e.pending())
    };
    assert!(attackers.contains(&bears), "the Bears may attack");
    assert!(!attackers.contains(&necro), "a defender may not");
}

#[test]
fn necropolis_exiling_a_creature_card_adds_its_mana_value_in_toughness_counters() {
    let mut e = table(&[
        card_index("c84adca4-19dd-432d-a363-e98d3b699af8"),
        serra_angel(),
    ]);
    to_graveyard(&mut e, serra_angel());
    let necro = on_battlefield(&e, P0, card_index("c84adca4-19dd-432d-a363-e98d3b699af8"))
        .expect("Necropolis is out");

    exile_for_counters(&mut e, serra_angel());

    assert_eq!(pt(&e, necro), (0, 6), "Serra Angel has mana value 5");
    assert!(
        in_graveyard(&e, P0, serra_angel()).is_none(),
        "it was exiled"
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Exile(P0)).len(), 1);
}

#[test]
fn necropolis_counters_from_two_activations_add_up() {
    let mut e = table(&[
        card_index("c84adca4-19dd-432d-a363-e98d3b699af8"),
        serra_angel(),
        grizzly_bears(),
    ]);
    to_graveyard(&mut e, serra_angel());
    to_graveyard(&mut e, grizzly_bears());
    let necro = on_battlefield(&e, P0, card_index("c84adca4-19dd-432d-a363-e98d3b699af8"))
        .expect("Necropolis is out");

    exile_for_counters(&mut e, grizzly_bears());
    assert_eq!(pt(&e, necro), (0, 3), "Bears have mana value 2");
    exile_for_counters(&mut e, serra_angel());
    assert_eq!(pt(&e, necro), (0, 8), "plus Serra Angel's five");
}

#[test]
fn necropolis_cannot_exile_a_noncreature_card() {
    let mut e = table(&[card_index("c84adca4-19dd-432d-a363-e98d3b699af8"), forest()]);
    assert!(
        !offers(&e, card_index("c84adca4-19dd-432d-a363-e98d3b699af8"), 0),
        "an empty graveyard offers nothing"
    );
    to_graveyard(&mut e, forest());
    assert!(in_graveyard(&e, P0, forest()).is_some());
    assert!(
        !offers(&e, card_index("c84adca4-19dd-432d-a363-e98d3b699af8"), 0),
        "a land card is not a creature card"
    );
}
