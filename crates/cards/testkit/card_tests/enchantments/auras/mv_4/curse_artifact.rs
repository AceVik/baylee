//! `cards/enchantments/auras/mv_4/curse_artifact.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

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

fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    priority(e, p);
    mana(e);
    let card = in_hand(e, p, c).expect("the spell is in hand");
    e.apply(p, PlayerAction::CastSpell { card })
        .expect("the spell is cast");
}

fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects,
            players: vec![],
        },
    )
    .expect("the targets are on the menu");
}

fn choose_objects(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(p, PlayerAction::ChooseObjects { objects })
        .expect("the choice is on the menu");
}

fn curse_artifact() -> CardIndex {
    card_index("f08ae501-8b96-40ee-aa7b-c25293ddfa88")
}

/// Curse Artifact on P1's Sol Ring, waiting at P1's first upkeep for the
/// question.
fn curse_at_the_upkeep_question() -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = prepared(&[curse_artifact()], &[], &[sol_ring()]);
    let ring = on_battlefield(&e, P1, sol_ring()).expect("P1's Sol Ring is out");
    cast(&mut e, P0, curse_artifact());
    targets(&mut e, vec![ring]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, curse_artifact()).is_some(),
        "the Aura is out"
    );

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, P1, "the artifact's controller is asked");
    assert_eq!(
        options,
        vec![ring],
        "the enchanted artifact may be sacrificed"
    );
    (e, ring)
}

/// Curse Artifact (f08ae501-8b96-40ee-aa7b-c25293ddfa88) — "Enchant artifact.
/// At the beginning of the upkeep of enchanted artifact's controller, this
/// Aura deals 2 damage to that player unless they sacrifice that artifact."
/// (declined)
#[test]
fn curse_artifact_deals_two_damage_to_the_artifacts_controller() {
    let (mut e, ring) = curse_at_the_upkeep_question();
    choose_objects(&mut e, vec![]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().players[1].life, 18, "two damage to P1");
    assert_eq!(
        e.state().players[0].life,
        20,
        "and none to the Aura's caster"
    );
    assert_eq!(e.state().object(ring).unwrap().zone, Zone::Battlefield);
    assert!(on_battlefield(&e, P0, curse_artifact()).is_some());
}

/// Curse Artifact — (sacrificed)
#[test]
fn curse_artifact_spares_the_player_who_sacrifices_the_artifact() {
    let (mut e, ring) = curse_at_the_upkeep_question();
    choose_objects(&mut e, vec![ring]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().players[1].life, 20, "no damage");
    assert!(in_graveyard(&e, P1, sol_ring()).is_some(), "sacrificed");
    assert!(
        on_battlefield(&e, P0, curse_artifact()).is_none(),
        "the Aura leaves with its artifact"
    );
}
