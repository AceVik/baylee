//! `cards/enchantments/auras/mv_3/farrel_s_mantle.rs`, played.

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

fn yes_no(e: &mut Engine<RegistryLookup>, answer: bool) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(p, PlayerAction::YesNo(answer))
        .expect("the question takes a yes or a no");
}

fn farrel_s_mantle() -> CardIndex {
    card_index("ffae3ff4-67f7-4e41-b166-c63f88c7aa08")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

/// P0's Grizzly Bears (2/2) wears Farrel's Mantle and attacks P1, who has only
/// a tapped Serra Angel (4/4) and so cannot block. Stops at the Mantle's
/// target question, returning the engine, the Bears and the Angel.
fn bears_attack_unblocked() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = prepared(&[farrel_s_mantle()], &[grizzly_bears()], &[serra_angel()]);
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    let angel = on_battlefield(&e, P1, serra_angel()).expect("Serra Angel is out");
    e.state.set_tapped(angel, true);

    cast(&mut e, P0, farrel_s_mantle());
    targets(&mut e, vec![bears]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, farrel_s_mantle()).is_some(),
        "the Aura is out"
    );

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P1))],
        },
    )
    .expect("the Bears attack");
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareBlockers);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("nobody blocks");
    (e, bears, angel)
}

/// Farrel's Mantle (ffae3ff4-67f7-4e41-b166-c63f88c7aa08) — "Enchant creature.
/// Whenever enchanted creature attacks and isn't blocked, its controller may
/// have it deal damage equal to its power plus 2 to another target creature. If
/// that player does, the attacking creature assigns no combat damage this turn."
/// (accepted)
#[test]
fn farrel_s_mantle_deals_power_plus_two_and_the_attacker_deals_no_combat_damage() {
    let (mut e, bears, angel) = bears_attack_unblocked();

    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the Mantle asks for a target, got {:?}", e.pending())
    };
    assert!(options.contains(&angel), "another creature may be named");
    assert!(!options.contains(&bears), "the enchanted creature may not");
    targets(&mut e, vec![angel]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut e, true);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(
        on_battlefield(&e, P1, serra_angel()).is_none(),
        "2 + 2 = 4 damage kills the 4/4 Angel"
    );
    assert_eq!(
        e.state().players[1].life,
        20,
        "the Bears assigned no combat damage"
    );
}

/// Farrel's Mantle — (declined: "may")
#[test]
fn farrel_s_mantle_declined_leaves_the_combat_damage_alone() {
    let (mut e, _bears, angel) = bears_attack_unblocked();

    targets(&mut e, vec![angel]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut e, false);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(
        on_battlefield(&e, P1, serra_angel()).is_some(),
        "no damage was dealt to the Angel"
    );
    assert_eq!(
        e.state().object(angel).unwrap().damage,
        0,
        "the Angel is unmarked"
    );
    assert_eq!(e.state().players[1].life, 18, "the Bears hit for two");
}
