//! `cards/creatures/mv_9/leviathan.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Phase;

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

fn yes_no(e: &mut Engine<RegistryLookup>, answer: bool) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(p, PlayerAction::YesNo(answer))
        .expect("the question takes a yes or a no");
}

fn choose_objects(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(p, PlayerAction::ChooseObjects { objects })
        .expect("the choice is on the menu");
}

fn leviathan() -> CardIndex {
    card_index("d8c49932-9b4f-4cb0-a7e9-4b7eb12203ee")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Leviathan cast from hand in P0's first main phase with `islands` Islands
/// (and a Grizzly Bears, so that a declare-attackers question exists) in play.
fn leviathan_in_play(islands: usize) -> (Engine<RegistryLookup>, ObjectId) {
    let mut mine = vec![grizzly_bears()];
    mine.extend(std::iter::repeat_n(island(), islands));
    let mut e = prepared(&[leviathan()], &mine, &[]);
    cast(&mut e, P0, leviathan());
    pass_until(&mut e, stack_is_empty);
    let levi = on_battlefield(&e, P0, leviathan()).expect("Leviathan is out");
    (e, levi)
}

/// Waits for P0's upkeep question ("you may sacrifice two Islands") of `turn`.
fn wait_for_the_upkeep_question(e: &mut Engine<RegistryLookup>, turn: u32) {
    pass_until(e, |e| {
        e.state().turn.number == turn && matches!(e.pending(), Pending::YesNo { player: P0, .. })
    });
}

/// Says yes to the upkeep question and sacrifices the first two Islands it
/// offers; the Leviathan then untaps.
fn sacrifice_two_islands(e: &mut Engine<RegistryLookup>) {
    yes_no(e, true);
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending().clone()
    else {
        panic!("the sacrifice is asked next, got {:?}", e.pending())
    };
    assert_eq!((min, max), (2, 2), "exactly two Islands");
    assert!(options.len() >= 2);
    choose_objects(e, options[..2].to_vec());
    pass_until(e, stack_is_empty);
}

/// Leviathan (d8c49932-9b4f-4cb0-a7e9-4b7eb12203ee) — "Trample"
/// Leviathan — "This creature enters tapped and doesn't untap during your untap
/// step. At the beginning of your upkeep, you may sacrifice two Islands. If you
/// do, untap this creature." (declined)
#[test]
fn leviathan_enters_tapped_and_stays_tapped_when_the_islands_are_kept() {
    let (mut e, levi) = leviathan_in_play(2);
    assert!(is_tapped(&e, levi), "enters tapped");

    wait_for_the_upkeep_question(&mut e, 3);
    assert!(is_tapped(&e, levi), "the untap step did not untap it");
    yes_no(&mut e, false);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, levi), "declined: still tapped");
    assert_eq!(lands_of(&e, P0).len(), 2, "both Islands kept");

    wait_for_the_upkeep_question(&mut e, 5);
    assert!(
        is_tapped(&e, levi),
        "and the next untap step does not either"
    );
}

/// Leviathan — (accepted)
#[test]
fn leviathan_untaps_when_two_islands_are_sacrificed_at_upkeep() {
    let (mut e, levi) = leviathan_in_play(3);
    wait_for_the_upkeep_question(&mut e, 3);
    sacrifice_two_islands(&mut e);

    assert!(!is_tapped(&e, levi), "untapped");
    assert_eq!(lands_of(&e, P0).len(), 1, "two of three Islands are gone");
}

/// Leviathan — "This creature can't attack unless you sacrifice two Islands.
/// (This cost is paid as attackers are declared.)" (two to spare)
#[test]
fn leviathan_attacks_for_ten_by_sacrificing_two_more_islands() {
    let (mut e, levi) = leviathan_in_play(4);
    wait_for_the_upkeep_question(&mut e, 3);
    sacrifice_two_islands(&mut e);
    assert_eq!(lands_of(&e, P0).len(), 2);

    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(attackers.contains(&levi), "two Islands are left to pay");
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(levi, Defender::Player(P1))],
        },
    )
    .expect("Leviathan attacks");
    assert_eq!(
        lands_of(&e, P0).len(),
        0,
        "the attack cost the other two Islands"
    );

    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().players[1].life, 10, "ten unblocked damage");
}

/// Leviathan — (one Island left: it cannot pay)
#[test]
fn leviathan_cannot_attack_with_fewer_than_two_islands_to_sacrifice() {
    let (mut e, levi) = leviathan_in_play(3);
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    wait_for_the_upkeep_question(&mut e, 3);
    sacrifice_two_islands(&mut e);
    assert!(!is_tapped(&e, levi));
    assert_eq!(lands_of(&e, P0).len(), 1);

    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(attackers.contains(&bears), "the Bears may attack");
    assert!(!attackers.contains(&levi), "one Island cannot pay for two");
    assert!(
        e.apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(levi, Defender::Player(P1))],
            },
        )
        .is_err(),
        "the declaration is refused"
    );
}
