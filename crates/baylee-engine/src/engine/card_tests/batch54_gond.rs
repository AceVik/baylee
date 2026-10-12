//! Gond acceptance uses real entries and explicit replacement-order answers.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::{generated::index, mana::ManaPool};
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
mod entry;
mod mana;
fn obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn next_main(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == p
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(),Pending::Priority{player,..}if *player==p)
    });
}
fn ordered_entry(
    e: &mut Engine<RegistryLookup>,
    p: PlayerId,
    card: CardIndex,
    order: &[ObjectId],
) -> ObjectId {
    let id = in_hand(e, p, card).unwrap();
    e.apply(p, PlayerAction::PlayLand { card: id }).unwrap();
    for &source in order {
        if let Pending::ChooseCards {
            player,
            options,
            prompt: crate::choice::ChoicePrompt::EntryReplacement,
            ..
        } = e.pending()
        {
            assert_eq!(*player, p);
            assert!(options.contains(&source), "{source:?} not in {options:?}");
            e.apply(
                p,
                PlayerAction::ChooseObjects {
                    objects: vec![source],
                },
            )
            .unwrap();
        }
    }
    assert!(
        !matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::EntryReplacement,
                ..
            }
        ),
        "explicit order must finish all competing replacements"
    );
    assert_eq!(e.state.object(id).unwrap().zone, Zone::Battlefield);
    id
}
fn clear_pool(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        p.mana_pool = ManaPool::new();
    }
    e.refresh_offer();
}
