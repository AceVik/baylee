//! Whole-batch regressions for dependent searches, naming and entry choices.
#[allow(clippy::wildcard_imports)]
use super::*;
use super::{
    counter_resources::prepared,
    library_choices::{choose, library, searched},
};
use crate::event::GameEvent;
use baylee_core::generated::{index, subtypes};
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

mod commander;
mod forms;
mod graveyard;
mod naming;
mod searches;

fn target(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let who = e.pending().asked().unwrap();
    e.apply(who, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}
fn name(e: &mut Engine<RegistryLookup>, card: CardIndex) {
    let who = e.pending().asked().unwrap();
    e.apply(who, PlayerAction::ChooseCardName { card, face: 0 })
        .unwrap();
}
fn first_option(e: &Engine<RegistryLookup>) -> ObjectId {
    let Pending::ChooseCards { options, .. } = e.pending() else {
        panic!("{:?}", e.pending());
    };
    options[0]
}
fn yes(e: &mut Engine<RegistryLookup>, answer: bool) {
    let who = e.pending().asked().unwrap();
    e.apply(who, PlayerAction::YesNo(answer)).unwrap();
}
fn next_upkeep(e: &mut Engine<RegistryLookup>) {
    pass_until(e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == crate::turn::Step::Upkeep
    });
}

fn copy_rules_fixture(
    e: &mut Engine<RegistryLookup>,
    source: ObjectId,
    target: ObjectId,
    temporary: bool,
) {
    // Isolate a copy acquisition while a real card's linked trigger is pending.
    assert!(crate::copiable_abilities::apply_copy(
        &mut e.state,
        crate::copiable_abilities::CopyApplication {
            source,
            target,
            mods: &[],
            copy_index: 0,
            until_eot: temporary,
            resolving: None,
            text: crate::text_changes::TextChangeMap::IDENTITY,
        }
    ));
    e.sync_static_effects();
    e.state.refresh_characteristics();
    e.refresh_offer();
}
