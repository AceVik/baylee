//! `cards/enchantments/auras/mv_1/brainwash.rs`, played.

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

fn brainwash() -> CardIndex {
    card_index("5a81755f-bc44-47ac-8039-8847a5e37922")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// The attackers the active player's declare-attackers question offers.
fn offered_attackers(e: &mut Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    pass_until(e, |e| {
        e.state().turn.active == seat && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    attackers
}

/// Brainwash (5a81755f-bc44-47ac-8039-8847a5e37922) — "Enchant creature.
/// Enchanted creature can't attack unless its controller pays {3}."
#[test]
fn brainwash_makes_the_enchanted_creature_pay_three_to_attack() {
    let mut e = prepared(&[brainwash()], &[grizzly_bears(), savannah_lions()], &[]);
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    let lions = on_battlefield(&e, P0, savannah_lions()).expect("Savannah Lions is out");

    cast(&mut e, P0, brainwash());
    targets(&mut e, vec![bears]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, brainwash()).is_some(),
        "the Aura is out"
    );

    // Nothing floating: the enchanted creature is not offered, the other is.
    let attackers = offered_attackers(&mut e, P0);
    assert!(attackers.contains(&lions), "the Lions attack for free");
    assert!(!attackers.contains(&bears), "the Bears cannot pay {{3}}");
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert!(
        e.apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .is_err(),
        "no mana, no attack"
    );

    // Paying the {3} is the card's documented gap (`Coverage::Partial`):
    // the question offers no mana abilities (CR 508.1i), and the pool has
    // emptied by the time it is asked, so the Bears stay home.
}

/// Brainwash — the controller of the enchanted creature pays, whoever cast the Aura.
#[test]
fn brainwash_taxes_the_opponents_creature_on_their_turn() {
    let mut e = prepared(&[brainwash()], &[], &[grizzly_bears(), savannah_lions()]);
    let bears = on_battlefield(&e, P1, grizzly_bears()).expect("Grizzly Bears is out");
    let lions = on_battlefield(&e, P1, savannah_lions()).expect("Savannah Lions is out");

    cast(&mut e, P0, brainwash());
    targets(&mut e, vec![bears]);
    pass_until(&mut e, stack_is_empty);

    let attackers = offered_attackers(&mut e, P1);
    assert!(attackers.contains(&lions));
    assert!(
        !attackers.contains(&bears),
        "P1 has no {{3}} for the enchanted Bears"
    );
}
