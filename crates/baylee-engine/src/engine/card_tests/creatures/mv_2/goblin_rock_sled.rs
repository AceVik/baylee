//! `cards/creatures/mv_2/goblin_rock_sled.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn ready(mine: &[CardIndex], theirs: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut all = vec![card_index("ada3247e-ec5e-499d-bc15-1d9dd80a59ae")];
    all.extend_from_slice(mine);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &all)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let sled = on_battlefield(&e, P0, card_index("ada3247e-ec5e-499d-bc15-1d9dd80a59ae"))
        .expect("the Sled is out");
    (e, sled)
}

fn attackers_offered(e: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    pass_until(e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::DeclareAttackers
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        panic!("expected the attack question, got {:?}", e.pending())
    };
    attackers
}

fn is_tapped(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .expect("the Sled is out")
        .status
        .contains(Status::TAPPED)
}

/// Goblin Rock Sled (ada3247e-ec5e-499d-bc15-1d9dd80a59ae) — "Trample. This
/// creature doesn't untap during your untap step if it attacked during your
/// last turn. This creature can't attack unless defending player controls a
/// Mountain."
#[test]
fn goblin_rock_sled_cannot_attack_a_player_without_a_mountain() {
    let (mut e, sled) = ready(&[grizzly_bears()], &[forest()]);
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("the Bears are out");
    let offered = attackers_offered(&mut e);
    assert!(offered.contains(&bears), "the Bears may attack");
    assert!(!offered.contains(&sled), "no Mountain, no Sled attack");
}

#[test]
fn goblin_rock_sled_can_attack_a_player_who_controls_a_mountain() {
    let (mut e, sled) = ready(&[grizzly_bears()], &[mountain()]);
    let offered = attackers_offered(&mut e);
    assert!(offered.contains(&sled));
}

#[test]
fn goblin_rock_sled_that_attacked_stays_tapped_for_one_untap_step() {
    let (mut e, sled) = ready(&[], &[mountain()]);
    let offered = attackers_offered(&mut e);
    assert!(offered.contains(&sled));
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(sled, Defender::Player(P1))],
        },
    )
    .expect("the Sled attacks");
    pass_until(&mut e, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });
    assert_eq!(e.state().players[1].life, 17, "3 unblocked damage");
    assert!(is_tapped(&e, sled), "attacking tapped it");

    // My next turn: it attacked during my last turn, so it stays tapped.
    pass_until(&mut e, |e| e.state().turn.active == P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    assert!(is_tapped(&e, sled), "it did not untap");

    // It did not attack that turn, so the turn after it untaps.
    pass_until(&mut e, |e| e.state().turn.active == P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    assert!(!is_tapped(&e, sled), "it untapped");
}

#[test]
fn goblin_rock_sled_that_did_not_attack_untaps_normally() {
    let (mut e, sled) = ready(&[], &[mountain()]);
    e.state.set_tapped(sled, true);
    pass_until(&mut e, |e| e.state().turn.active == P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    assert!(!is_tapped(&e, sled), "it never attacked, so it untaps");
}
