//! `cards/enchantments/auras/mv_3/tourach_s_gate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

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

fn activate_by_card(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex, ability_index: u32) {
    priority(e, p);
    mana(e);
    let source = on_battlefield(e, p, c).expect("the source is on the battlefield");
    e.apply(
        p,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .expect("the ability is activated");
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

fn tourach_s_gate() -> CardIndex {
    card_index("03e27ebd-6141-4002-b41f-0bf9d6ec2908")
}

fn basal_thrull() -> CardIndex {
    card_index("9da50130-3f83-4968-983c-7dcba257cf1b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn time_counters(e: &Engine<RegistryLookup>, gate: ObjectId) -> u16 {
    e.state()
        .object(gate)
        .expect("the Gate is out")
        .counters
        .get(CounterKind::Time)
}

/// Tourach's Gate on P0's Swamp, with a Thrull, a Bear and Lions at P0's side.
/// Returns the engine, the Gate and the Swamp.
fn gate_on_a_swamp() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = prepared(
        &[tourach_s_gate()],
        &[swamp(), basal_thrull(), grizzly_bears(), savannah_lions()],
        &[],
    );
    let land = on_battlefield(&e, P0, swamp()).expect("the Swamp is out");
    cast(&mut e, P0, tourach_s_gate());
    targets(&mut e, vec![land]);
    pass_until(&mut e, stack_is_empty);
    let gate = on_battlefield(&e, P0, tourach_s_gate()).expect("the Gate is out");
    (e, gate, land)
}

/// Tourach's Gate (03e27ebd-6141-4002-b41f-0bf9d6ec2908) — "Enchant land you
/// control. Sacrifice a Thrull: Put three time counters on this Aura."
#[test]
fn tourach_s_gate_sacrificing_a_thrull_puts_three_time_counters_on_it() {
    let (mut e, gate, _land) = gate_on_a_swamp();
    let thrull = on_battlefield(&e, P0, basal_thrull()).expect("Basal Thrull is out");
    assert_eq!(time_counters(&e, gate), 0);

    activate_by_card(&mut e, P0, tourach_s_gate(), 1);
    choose_objects(&mut e, vec![thrull]);
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, P0, basal_thrull()).is_some(), "sacrificed");
    assert_eq!(time_counters(&e, gate), 3);
}

/// Tourach's Gate — only a Thrull may be sacrificed to it.
#[test]
fn tourach_s_gate_asks_for_a_thrull_to_sacrifice() {
    let (mut e, _gate, _land) = gate_on_a_swamp();
    let thrull = on_battlefield(&e, P0, basal_thrull()).expect("Basal Thrull is out");
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    activate_by_card(&mut e, P0, tourach_s_gate(), 1);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("a sacrifice is asked, got {:?}", e.pending())
    };
    assert!(options.contains(&thrull));
    assert!(!options.contains(&bears), "a Bear is no Thrull");
}

/// Tourach's Gate — "At the beginning of your upkeep, remove a time counter
/// from this Aura. If there are no time counters on this Aura, sacrifice it."
#[test]
fn tourach_s_gate_counts_down_and_is_sacrificed_when_none_are_left() {
    let (mut e, gate, _land) = gate_on_a_swamp();
    let thrull = on_battlefield(&e, P0, basal_thrull()).expect("Basal Thrull is out");
    activate_by_card(&mut e, P0, tourach_s_gate(), 1);
    choose_objects(&mut e, vec![thrull]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(time_counters(&e, gate), 3);

    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && time_counters(e, gate) == 2
    });
    pass_until(&mut e, |e| {
        e.state().turn.number == 5 && time_counters(e, gate) == 1
    });
    // The last counter comes off at turn 7's upkeep and the Aura goes.
    pass_until(&mut e, |e| {
        e.state().turn.number == 7 && on_battlefield(e, P0, tourach_s_gate()).is_none()
    });
    assert!(
        in_graveyard(&e, P0, tourach_s_gate()).is_some(),
        "sacrificed"
    );
}

/// Tourach's Gate — (no counters: the first upkeep sacrifices it)
#[test]
fn tourach_s_gate_without_counters_is_sacrificed_at_the_next_upkeep() {
    let (mut e, _gate, _land) = gate_on_a_swamp();
    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && on_battlefield(e, P0, tourach_s_gate()).is_none()
    });
    assert!(
        in_graveyard(&e, P0, tourach_s_gate()).is_some(),
        "sacrificed"
    );
}

/// Tourach's Gate — "Tap enchanted land: Attacking creatures you control get
/// +2/-1 until end of turn. Activate only if enchanted land is untapped."
#[test]
fn tourach_s_gate_taps_the_land_to_pump_attackers_only() {
    let (mut e, _gate, land) = gate_on_a_swamp();
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    let lions = on_battlefield(&e, P0, savannah_lions()).expect("Savannah Lions is out");

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P1))],
        },
    )
    .expect("the Bears attack");
    assert_eq!(pt(&e, bears), (2, 2));

    activate_by_card(&mut e, P0, tourach_s_gate(), 3);
    choose_objects(&mut e, vec![land]);
    pass_until(&mut e, stack_is_empty);

    assert!(is_tapped(&e, land), "the enchanted land was tapped");
    assert_eq!(pt(&e, bears), (4, 1), "the attacker gets +2/-1");
    assert_eq!(
        pt(&e, lions),
        (2, 1),
        "a creature that is not attacking does not"
    );
}

/// Tourach's Gate — (the land must be untapped)
#[test]
fn tourach_s_gate_pump_needs_an_untapped_land() {
    let (mut e, gate, land) = gate_on_a_swamp();
    assert!(priority_offer(&e).abilities.contains(&(gate, 3)));
    e.state.set_tapped(land, true);
    e.refresh_offer();
    assert!(
        !priority_offer(&e).abilities.contains(&(gate, 3)),
        "a tapped land cannot be tapped again"
    );
}
