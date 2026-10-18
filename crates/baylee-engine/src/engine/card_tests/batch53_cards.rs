//! Behavioral card tests for Batch 53 (28 newly implemented cards + 3 verified).

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::ChoicePrompt;
use crate::event::Cause;
use crate::turn::{Phase, Step};
use crate::zone::ZonePosition;
use baylee_core::generated::{index, subtypes};

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
