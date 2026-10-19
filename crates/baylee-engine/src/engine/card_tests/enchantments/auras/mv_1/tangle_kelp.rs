//! `cards/enchantments/auras/mv_1/tangle_kelp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};

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

fn tangle_kelp() -> CardIndex {
    card_index("06ba313f-50b4-4e84-a8e0-b6dda47225f2")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Tangle Kelp (06ba313f-50b4-4e84-a8e0-b6dda47225f2) — "Enchant creature.
/// When this Aura enters, tap enchanted creature." (and a creature that did
/// not attack untaps as usual)
#[test]
fn tangle_kelp_taps_the_creature_when_it_enters_and_it_untaps_if_it_did_not_attack() {
    let mut e = prepared(&[tangle_kelp()], &[], &[hill_giant()]);
    let giant = on_battlefield(&e, P1, hill_giant()).expect("Hill Giant is out");
    assert!(!is_tapped(&e, giant));

    cast(&mut e, P0, tangle_kelp());
    targets(&mut e, vec![giant]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, tangle_kelp()).is_some(),
        "the Aura is out"
    );
    assert!(is_tapped(&e, giant), "tapped as the Aura entered");

    // It did not attack during its controller's last turn (it had none), so
    // its controller's untap step untaps it.
    pass_until(&mut e, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep
    });
    assert!(!is_tapped(&e, giant), "untaps in turn 2");
}

/// Tangle Kelp — "Enchanted creature doesn't untap during its controller's
/// untap step if it attacked during its controller's last turn."
#[test]
fn tangle_kelp_keeps_a_creature_tapped_for_one_untap_step_after_it_attacked() {
    let mut e = prepared(&[tangle_kelp()], &[], &[hill_giant()]);
    let giant = on_battlefield(&e, P1, hill_giant()).expect("Hill Giant is out");

    // Turn 2: the Giant attacks P0.
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(giant, Defender::Player(P0))],
        },
    )
    .expect("the Giant attacks");

    // Turn 3: P0 enchants it.
    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    cast(&mut e, P0, tangle_kelp());
    targets(&mut e, vec![giant]);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, giant));

    // Turn 4: it attacked in its controller's last turn, so it stays tapped.
    pass_until(&mut e, |e| {
        e.state().turn.number == 4 && e.state().turn.step == Step::Upkeep
    });
    assert!(is_tapped(&e, giant), "did not untap after attacking");

    // Turn 6: it did not attack in turn 4, so it untaps.
    pass_until(&mut e, |e| {
        e.state().turn.number == 6 && e.state().turn.step == Step::Upkeep
    });
    assert!(!is_tapped(&e, giant), "untaps once it has not attacked");
}
