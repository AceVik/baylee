//! `cards/creatures/mv_6/feral_thallid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);

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

fn feral_thallid() -> CardIndex {
    card_index("3953fd17-03f4-4431-99b5-87dcac5a279d")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn spores(e: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    e.state()
        .object(id)
        .expect("the Thallid is out")
        .counters
        .get(counters::SPORE)
}

/// Feral Thallid (3953fd17-03f4-4431-99b5-87dcac5a279d) — "At the beginning of
/// your upkeep, put a spore counter on this creature."
#[test]
fn feral_thallid_gains_a_spore_counter_at_each_of_its_controllers_upkeeps() {
    let mut e = prepared(&[], &[feral_thallid()], &[]);
    let thallid = on_battlefield(&e, P0, feral_thallid()).expect("Feral Thallid is out");
    assert_eq!(pt(&e, thallid), (6, 3));
    assert_eq!(spores(&e, thallid), 1, "turn 1's upkeep");

    pass_until(&mut e, |e| {
        e.state().turn.number == 2 && e.state().turn.phase == crate::turn::Phase::FirstMain
    });
    assert_eq!(spores(&e, thallid), 1, "the opponent's upkeep adds nothing");

    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && spores(e, thallid) == 2
    });
    pass_until(&mut e, |e| {
        e.state().turn.number == 5 && spores(e, thallid) == 3
    });
}

/// Feral Thallid — "Remove three spore counters from this creature: Regenerate
/// this creature."
#[test]
fn feral_thallid_needs_three_spore_counters_to_regenerate() {
    let mut e = prepared(&[lightning_bolt()], &[feral_thallid()], &[]);
    let thallid = on_battlefield(&e, P0, feral_thallid()).expect("Feral Thallid is out");

    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && spores(e, thallid) == 2
    });
    assert!(
        !priority_offer(&e).abilities.contains(&(thallid, 1)),
        "two counters pay nothing"
    );

    pass_until(&mut e, |e| {
        e.state().turn.number == 5 && spores(e, thallid) == 3
    });
    assert!(
        priority_offer(&e).abilities.contains(&(thallid, 1)),
        "three counters pay"
    );
    activate_by_card(&mut e, P0, feral_thallid(), 1);
    assert_eq!(spores(&e, thallid), 0, "the three counters are the cost");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(thallid).unwrap().regeneration_shields,
        1,
        "a regeneration shield stands"
    );

    // Lethal damage: the shield replaces the destruction.
    cast(&mut e, P0, lightning_bolt());
    targets(&mut e, vec![thallid]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, feral_thallid()).is_some(),
        "regenerated instead of dying"
    );
    assert!(is_tapped(&e, thallid), "regeneration taps it");
    assert_eq!(
        e.state().object(thallid).unwrap().damage,
        0,
        "and removes the damage"
    );
}

/// Feral Thallid — without the shield, three damage kills the 6/3.
#[test]
fn feral_thallid_without_a_shield_dies_to_lightning_bolt() {
    let mut e = prepared(&[lightning_bolt()], &[feral_thallid()], &[]);
    let thallid = on_battlefield(&e, P0, feral_thallid()).expect("Feral Thallid is out");
    cast(&mut e, P0, lightning_bolt());
    targets(&mut e, vec![thallid]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, feral_thallid()).is_none());
    assert!(in_graveyard(&e, P0, feral_thallid()).is_some());
}
