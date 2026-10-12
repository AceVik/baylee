//! Printed linked reanimation families, distinct from delayed triggers.
use super::{Resolution, subjects};
use crate::{
    event::{GameEvent, JournalEntry},
    object::{GameObject, Rider},
    state::GameState,
    zone::Zone,
};
use baylee_core::ids::DamageSourceRef;

#[derive(Clone, Copy, Debug, Hash)]
pub(super) struct PendingLink {
    pub source: DamageSourceRef,
    pub creature: DamageSourceRef,
    pub acquisition: u64,
}

pub(super) fn queue(state: &GameState, res: &mut Resolution, creature: DamageSourceRef) {
    if let Some(source) =
        subjects::source(state, res).filter(|source| subjects::on_battlefield(state, *source))
    {
        res.subject.reanimation_links.push(PendingLink {
            source,
            creature,
            acquisition: subjects::acquisition(state, res),
        });
    }
}

pub(super) fn flush(state: &mut GameState, res: &mut Resolution) {
    let mut waiting = Vec::new();
    for link in std::mem::take(&mut res.subject.reanimation_links) {
        if state.entries.contains(link.creature.object) {
            waiting.push(link);
            continue;
        }
        if subjects::on_battlefield(state, link.creature)
            && subjects::on_battlefield(state, link.source)
            && let Some(source) = state.object_mut(link.source.object)
        {
            let rider = Rider::ReanimatedCreature {
                creature: link.creature,
                acquisition: link.acquisition,
            };
            if !source.riders.contains(&rider) {
                source.riders.push(rider);
            }
            if !state.reanimation_sources.contains(&link.source) {
                state.reanimation_sources.push(link.source);
            }
        }
    }
    res.subject.reanimation_links = waiting;
}

pub(crate) fn departure(source: &GameObject, acquisition: u64, entry: &JournalEntry) -> u32 {
    let GameEvent::ZoneChanged {
        object,
        from: Zone::Battlefield,
        to,
        ..
    } = entry.event
    else {
        return 0;
    };
    if to == Zone::Battlefield {
        return 0;
    }
    let Some(departure) = &entry.departure else {
        return 0;
    };
    u32::from(source.riders.contains(&Rider::ReanimatedCreature {
        creature: DamageSourceRef {
            object,
            version: departure.version,
        },
        acquisition,
    }))
}

/// Once all departures have been observed, dead links cannot trigger again.
pub(crate) fn prune(state: &mut GameState) {
    let mut sources = std::mem::take(&mut state.reanimation_sources);
    sources.retain(|source| {
        if !subjects::on_battlefield(state, *source) {
            return false;
        }
        // Temporarily take the sparse rider list so identity checks can read
        // the state while retaining in place, without an O(links²) dead list.
        let mut riders = std::mem::take(&mut state.object_mut(source.object).expect("current source").riders);
        riders.retain(|rider| !matches!(rider, Rider::ReanimatedCreature { creature, .. } if !subjects::on_battlefield(state, *creature)));
        let linked = riders.iter().any(|rider| matches!(rider, Rider::ReanimatedCreature { .. }));
        state.object_mut(source.object).expect("current source").riders = riders;
        linked
    });
    state.reanimation_sources = sources;
}
