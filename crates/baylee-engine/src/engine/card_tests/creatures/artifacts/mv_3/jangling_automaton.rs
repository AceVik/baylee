//! `cards/creatures/artifacts/mv_3/jangling_automaton.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::object::Status;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn jangling_automaton() -> CardIndex {
    card_index("b6f67539-9f76-42b2-96a8-575f27ba928e")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn tap(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    e.state
        .object_mut(id)
        .expect("object exists")
        .status
        .insert(Status::TAPPED);
}

/// Jangling Automaton (b6f67539-9f76-42b2-96a8-575f27ba928e) — "Whenever this
/// creature attacks, untap all creatures defending player controls." Both of
/// the defender's tapped creatures are untapped by the attack — which shows
/// at the block question, where they are offered as blockers — and the
/// attacker's own tapped Hill Giant stays tapped.
#[test]
fn jangling_automaton_untaps_every_creature_the_defender_controls_when_it_attacks() {
    let mut e = lk::game(
        SEED,
        &[jangling_automaton(), hill_giant()],
        &[grizzly_bears(), savannah_lions()],
        &[],
        &[],
    );
    let automaton = lk::obj(&e, P0, jangling_automaton());
    let giant = lk::obj(&e, P0, hill_giant());
    let bears = lk::obj(&e, P1, grizzly_bears());
    let lions = lk::obj(&e, P1, savannah_lions());
    for id in [giant, bears, lions] {
        tap(&mut e, id);
    }
    assert!(
        lk::tapped(&e, bears) && lk::tapped(&e, lions),
        "tapped before the attack"
    );

    let offer = lk::attack(&mut e, &[automaton]);

    assert!(
        !lk::tapped(&e, bears) && !lk::tapped(&e, lions),
        "untapped by the trigger"
    );
    assert!(
        lk::may_block(&offer, bears, automaton),
        "and offered as a blocker"
    );
    assert!(lk::may_block(&offer, lions, automaton));
    assert!(
        lk::tapped(&e, giant),
        "the attacker's own creatures are not untapped"
    );
}

/// Jangling Automaton — "whenever this creature *attacks*": before the attack
/// the defender's tapped creature is still tapped, and a Hill Giant attacking
/// instead of the Automaton untaps nothing.
#[test]
fn jangling_automaton_untaps_nothing_when_another_creature_attacks() {
    let mut e = lk::game(
        SEED,
        &[jangling_automaton(), hill_giant()],
        &[grizzly_bears()],
        &[],
        &[],
    );
    let giant = lk::obj(&e, P0, hill_giant());
    let bears = lk::obj(&e, P1, grizzly_bears());
    tap(&mut e, bears);

    let offer = lk::attack(&mut e, &[giant]);

    assert!(lk::tapped(&e, bears), "the Automaton did not attack");
    assert!(
        !lk::may_block(&offer, bears, giant),
        "a tapped creature cannot block"
    );
}
