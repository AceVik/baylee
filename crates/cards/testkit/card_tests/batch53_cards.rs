//! Behavioral card tests for Batch 53 (28 newly implemented cards + 3 verified).

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};
use baylee_engine::choice::ChoicePrompt;
use baylee_engine::event::Cause;
use baylee_engine::turn::{Phase, Step};
use baylee_engine::zone::ZonePosition;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

mod combat_and_activated_creatures;
mod draw_and_search_creatures;
mod enchantments_and_locks;
mod equipment_and_lands;
mod planeswalkers;
mod spells_and_counters;

fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
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

/// Plays a real blocked combat: nonlethal printed damage is lethal with deathtouch.
#[track_caller]
fn assert_deathtouch_kills_larger_blocker(card: CardIndex) {
    let mut e = prepared(&[], &[card], &[index::ROOTBREAKER_WURM]);
    let attacker = on_battlefield(&e, P0, card).unwrap();
    let blocker = on_battlefield(&e, P1, index::ROOTBREAKER_WURM).unwrap();
    assert!(pt(&e, attacker).0 < pt(&e, blocker).1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(P1))],
        },
    )
    .unwrap();
    for _ in 0..40 {
        match e.pending().clone() {
            Pending::ChooseBlockers { .. } => break,
            Pending::YesNo { player, .. } => {
                e.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected before blockers: {other:?}"),
        }
    }
    assert!(matches!(e.pending(), Pending::ChooseBlockers { .. }));
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, attacker)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(in_graveyard(&e, P1, index::ROOTBREAKER_WURM).is_some());
    assert!(in_graveyard(&e, P0, card).is_some());
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
    let card = in_hand(e, p, c).unwrap();
    e.apply(p, PlayerAction::CastSpell { card }).unwrap();
}

fn activate_by_card(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex, ability_index: u32) {
    priority(e, p);
    mana(e);
    let source = on_battlefield(e, p, c).unwrap();
    e.apply(
        p,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .unwrap();
}

fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}

fn choose_objects(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseObjects { objects }).unwrap();
}

fn yes_no(e: &mut Engine<RegistryLookup>, answer: bool) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::YesNo(answer)).unwrap();
}
