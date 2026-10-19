//! `cards/creatures/mv_5/orgg.rs`, played.

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

fn orgg() -> CardIndex {
    card_index("f6ea2b1b-1403-4ccd-a23c-48e0e81ceb91")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// The attackers P0's declare-attackers question offers.
fn offered_attackers(e: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    pass_until(e, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    attackers
}

/// Orgg (f6ea2b1b-1403-4ccd-a23c-48e0e81ceb91) — "Trample"
#[test]
fn orgg_is_a_six_six_with_trample() {
    let e = prepared(&[], &[orgg()], &[]);
    let big = on_battlefield(&e, P0, orgg()).expect("Orgg is out");
    assert_eq!(pt(&e, big), (6, 6));
    assert!(keywords(&e, big).contains(KeywordSet::TRAMPLE));
}

/// Orgg — "This creature can't attack if defending player controls an untapped
/// creature with power 3 or greater."
#[test]
fn orgg_cannot_attack_into_an_untapped_creature_with_power_three() {
    // Hill Giant (3/3) untapped: Orgg is not offered and the declaration is refused.
    let mut e = prepared(&[], &[orgg(), grizzly_bears()], &[hill_giant()]);
    let big = on_battlefield(&e, P0, orgg()).expect("Orgg is out");
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    let attackers = offered_attackers(&mut e);
    assert!(attackers.contains(&bears), "the Bears may attack");
    assert!(
        !attackers.contains(&big),
        "Orgg may not attack past an untapped Hill Giant"
    );
    assert!(
        e.apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(big, Defender::Player(P1))],
            },
        )
        .is_err(),
        "the declaration is refused"
    );

    // The same Giant, tapped: Orgg attacks.
    let mut e = prepared(&[], &[orgg(), grizzly_bears()], &[hill_giant()]);
    let big = on_battlefield(&e, P0, orgg()).expect("Orgg is out");
    let giant = on_battlefield(&e, P1, hill_giant()).expect("Hill Giant is out");
    e.state.set_tapped(giant, true);
    let attackers = offered_attackers(&mut e);
    assert!(
        attackers.contains(&big),
        "a tapped Giant does not stop Orgg"
    );
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(big, Defender::Player(P1))],
        },
    )
    .expect("Orgg attacks");

    // An untapped creature with power 2 does not stop it either.
    let mut e = prepared(&[], &[orgg()], &[grizzly_bears()]);
    let big = on_battlefield(&e, P0, orgg()).expect("Orgg is out");
    let attackers = offered_attackers(&mut e);
    assert!(
        attackers.contains(&big),
        "power 2 is below the line: {attackers:?}"
    );
}

/// Orgg — "This creature can't block creatures with power 3 or greater."
#[test]
fn orgg_cannot_block_a_creature_with_power_three_but_blocks_a_smaller_one() {
    let mut e = prepared(&[], &[orgg()], &[hill_giant(), grizzly_bears()]);
    let big = on_battlefield(&e, P0, orgg()).expect("Orgg is out");
    let giant = on_battlefield(&e, P1, hill_giant()).expect("Hill Giant is out");
    let bears = on_battlefield(&e, P1, grizzly_bears()).expect("Grizzly Bears is out");

    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == crate::turn::Phase::FirstMain
    });
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(giant, Defender::Player(P0)), (bears, Defender::Player(P0))],
        },
    )
    .expect("both attack");
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = e.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    let option = blockers
        .iter()
        .find(|b| b.blocker == big)
        .expect("Orgg may block something");
    assert!(
        option.attackers.contains(&bears),
        "Orgg may block the Bears"
    );
    assert!(
        !option.attackers.contains(&giant),
        "Orgg may not block the Hill Giant"
    );
    assert!(
        e.apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(big, giant)],
            },
        )
        .is_err(),
        "blocking the Giant is refused"
    );
    e.apply(
        P0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(big, bears)],
        },
    )
    .expect("blocking the Bears is fine");
}
