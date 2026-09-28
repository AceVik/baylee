//! Courser's permissions and the CR 401.5 information boundary.
//! <https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf>
use super::{testkit::*, *};
use crate::{
    event::{Cause, GameEvent},
    zone::{Zone, ZoneLocation, ZonePosition},
};
use baylee_core::ids::CardIndex;
const P0: PlayerId = PlayerId::new(0);
fn card(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).unwrap()
}
fn setup(extra: bool) -> Engine<RegistryLookup> {
    let mut field = vec![card("Courser of Kruphix")];
    if extra {
        field.push(card("Exploration"));
    }
    let mut e = Duel::table(985, card("Forest"), 3)
        .battlefield(0, &field)
        .hand(0, &[card("Breeding Pool"), card("Llanowar Elves")])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}
fn top(e: &Engine<RegistryLookup>) -> ObjectId {
    *e.state
        .zones
        .list(ZoneLocation::Library(P0))
        .last()
        .unwrap()
}
fn put_on_top(e: &mut Engine<RegistryLookup>, name: &str) -> ObjectId {
    let id = in_hand(e, P0, card(name)).unwrap();
    e.state
        .move_object(
            id,
            ZoneLocation::Library(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    id
}
#[test]
fn courser_plays_only_the_top_card_at_ordinary_land_timing_and_triggers_landfall() {
    let mut e = setup(false);
    let library = e.state.zones.list(ZoneLocation::Library(P0));
    let lower = library[library.len() - 2];
    let first = top(&e);
    let other = *e
        .state
        .zones
        .list(ZoneLocation::Library(PlayerId::new(1)))
        .last()
        .unwrap();
    assert!(e.state.library_top_revealed(P0));
    assert!(!e.state.library_top_revealed(PlayerId::new(1)));
    for illegal in [lower, other] {
        assert!(!crate::casting::land_card_open(&e.state, P0, illegal));
        assert!(
            e.apply(P0, PlayerAction::PlayLand { card: illegal })
                .is_err()
        );
        assert!(crate::casting::play_land(&mut e.state, P0, illegal).is_err());
    }
    let life = e.state.players[0].life;
    e.apply(P0, PlayerAction::PlayLand { card: first }).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(first).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.players[0].life, life + 1);
    assert_eq!(e.state.players[0].lands_played_this_turn, 1);
    assert!(
        e.apply(P0, PlayerAction::PlayLand { card: top(&e) })
            .is_err()
    );
    assert!(e.library_reveal_blocked().is_empty());
}
#[test]
fn courser_extra_land_permission_and_removal_are_rechecked() {
    let mut e = setup(true);
    for _ in 0..2 {
        let land = top(&e);
        e.apply(P0, PlayerAction::PlayLand { card: land }).unwrap();
        pass_until(&mut e, stack_is_empty);
    }
    assert!(
        e.apply(P0, PlayerAction::PlayLand { card: top(&e) })
            .is_err()
    );
    let courser = on_battlefield(&e, P0, card("Courser of Kruphix")).unwrap();
    e.state
        .move_object(
            courser,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    e.refresh_offer();
    assert!(!e.state.library_top_revealed(P0));
    assert!(!crate::casting::land_card_open(&e.state, P0, top(&e)));
}
#[test]
fn courser_nonland_top_is_visible_but_not_playable_or_castable() {
    let mut e = setup(false);
    let elf = put_on_top(&mut e, "Llanowar Elves");
    let legal = e.compute_legal(P0);
    assert!(!legal.lands.contains(&elf));
    assert!(!legal.castable.contains(&elf));
    assert!(e.apply(P0, PlayerAction::PlayLand { card: elf }).is_err());
    e.state.turn.phase = Phase::Combat;
    let forest = e.state.zones.list(ZoneLocation::Library(P0))[0];
    e.state
        .move_object(
            forest,
            ZoneLocation::Library(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert!(crate::casting::play_land(&mut e.state, P0, forest).is_err());
}
#[test]
fn courser_shockland_decision_cannot_peek_at_the_next_card() {
    let mut e = setup(false);
    let shock = put_on_top(&mut e, "Breeding Pool");
    assert!(e.library_reveal_blocked().is_empty());
    e.apply(P0, PlayerAction::PlayLand { card: shock }).unwrap();
    assert!(matches!(e.pending(), Pending::YesNo { .. }));
    assert!(e.library_reveal_blocked().contains(P0));
    let hash = e.snapshot_hash();
    let remembered = e.library_action_tops.take();
    assert_ne!(
        e.snapshot_hash(),
        hash,
        "visibility-changing continuation is replay state"
    );
    e.library_action_tops = remembered;
    assert_eq!(e.snapshot_hash(), hash);
    assert!(!e.library_reveal_blocked().contains(PlayerId::new(1)));
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert!(e.library_reveal_blocked().is_empty());
    assert!(
        e.state
            .object(shock)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}
#[test]
fn courser_multi_draw_records_each_revealed_card_without_revealing_lower_cards() {
    let mut e = setup(false);
    let library = e.state.zones.list(ZoneLocation::Library(P0)).clone();
    let start = e.state.journal.entries().len();
    let drawn = e.state.draw_cards(P0, 3);
    let shown: Vec<_> = e.state.journal.entries()[start..]
        .iter()
        .filter_map(|entry| match &entry.event {
            GameEvent::Revealed { cards, .. } => Some(cards.as_slice()),
            _ => None,
        })
        .flatten()
        .copied()
        .collect();
    assert_eq!(shown, drawn);
    assert_eq!(
        shown,
        library.iter().rev().take(3).copied().collect::<Vec<_>>()
    );
}
