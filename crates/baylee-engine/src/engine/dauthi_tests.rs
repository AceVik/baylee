//! Dauthi's September 28 reports: shadow and replacement of graveyard entry.
//! Rules source: <https://magic.wizards.com/en/news/feature/marvel-super-heroes-release-notes>.
use super::{testkit::*, *};
use crate::{
    event::{Cause, GameEvent},
    object::{Rider, Status},
    zone::{Zone, ZoneLocation, ZonePosition},
};
use baylee_cards_dsl::{Effect, Filter, counters::VOID};
use baylee_core::ids::CardIndex;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);
fn card(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).unwrap()
}
fn setup(team: bool) -> Engine<RegistryLookup> {
    let mut duel = Duel::table(997, card("Swamp"), 3)
        .battlefield(
            0,
            &[
                card("Dauthi Voidwalker"),
                card("Swamp"),
                card("Swamp"),
                card("Swamp"),
            ],
        )
        .battlefield(1, &[card("Llanowar Elves"), card("Swamp")])
        .battlefield(2, &[card("Ornithopter")])
        .hand(0, &[card("Mind Twist"), card("Toxic Deluge")])
        .hand(
            1,
            &[
                card("Dark Ritual"),
                card("Swamp"),
                card("Swamp"),
                card("Swamp"),
            ],
        )
        .hand(2, &[card("Swamp"); 4]);
    if team {
        duel = duel.team(0, 1).team(1, 1).team(2, 2);
    }
    let mut engine = duel.start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}
fn move_to(e: &mut Engine<RegistryLookup>, id: ObjectId, zone: ZoneLocation) {
    e.state
        .move_object(id, zone, ZonePosition::Top, Cause::Effect)
        .unwrap();
}
fn exiled(e: &Engine<RegistryLookup>, id: ObjectId) {
    let o = e.state.object(id).unwrap();
    assert_eq!(o.zone, Zone::Exile);
    assert_eq!(o.counters.get(VOID), 1);
}
#[test]
fn dauthi_replaces_cards_from_hand_library_and_battlefield_but_not_own_cards() {
    let mut e = setup(false);
    let hand = e.state.zones.list(ZoneLocation::Hand(P1))[0];
    let library = *e
        .state
        .zones
        .list(ZoneLocation::Library(P1))
        .last()
        .unwrap();
    let elf = on_battlefield(&e, P1, card("Llanowar Elves")).unwrap();
    for id in [hand, library, elf] {
        move_to(&mut e, id, ZoneLocation::Graveyard(P1));
        exiled(&e, id);
        assert!(
            !e.state
                .journal
                .entries()
                .iter()
                .any(|entry| matches!(entry.event,
            GameEvent::ZoneChanged { object, to: Zone::Graveyard, .. } if object == id))
        );
    }
    let own = e.state.zones.list(ZoneLocation::Hand(P0))[0];
    move_to(&mut e, own, ZoneLocation::Graveyard(P0));
    assert_eq!(e.state.object(own).unwrap().zone, Zone::Graveyard);
}
#[test]
fn dauthi_preserves_discard_events_without_putting_the_cards_in_the_graveyard() {
    let mut e = setup(false);
    cast_from_hand(&mut e, P0, card("Mind Twist"));
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    let before = e.state.zones.list(ZoneLocation::Hand(P1)).len();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.zones.list(ZoneLocation::Hand(P1)).len(), before - 2);
    let cards = e.state.zones.list(ZoneLocation::Exile(P1));
    assert_eq!(cards.len(), 2);
    for &id in cards {
        exiled(&e, id);
    }
    assert!(e.state.zones.list(ZoneLocation::Graveyard(P1)).is_empty());
    assert_eq!(
        e.state
            .journal
            .entries()
            .iter()
            .filter(
                |entry| matches!(entry.event, GameEvent::Discarded { player, .. } if player == P1)
            )
            .count(),
        2
    );
}
#[test]
fn dauthi_exiles_a_resolved_opponents_spell() {
    let mut e = setup(false);
    assert!(walk_to_own_main(&mut e, P1));
    let ritual = in_hand(&e, P1, card("Dark Ritual")).unwrap();
    cast_from_hand(&mut e, P1, card("Dark Ritual"));
    pass_until(&mut e, stack_is_empty);
    exiled(&e, ritual);
}
#[test]
fn dauthi_replacement_survives_simultaneous_deaths_then_stops_and_counters_do_not_follow() {
    let mut e = setup(false);
    let dauthi = on_battlefield(&e, P0, card("Dauthi Voidwalker")).unwrap();
    let elf = on_battlefield(&e, P1, card("Llanowar Elves")).unwrap();
    let thopter = on_battlefield(&e, P2, card("Ornithopter")).unwrap();
    cast_from_hand(&mut e, P0, card("Toxic Deluge"));
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(dauthi).unwrap().zone, Zone::Graveyard);
    exiled(&e, elf);
    exiled(&e, thopter);
    move_to(&mut e, elf, ZoneLocation::Hand(P1));
    assert_eq!(e.state.object(elf).unwrap().counters.get(VOID), 0);
    move_to(&mut e, elf, ZoneLocation::Graveyard(P1));
    assert_eq!(e.state.object(elf).unwrap().zone, Zone::Graveyard);
}
#[test]
fn dauthi_ignores_tokens_spell_copies_teammates_and_phased_out_sources() {
    let mut e = setup(true);
    let ally = e.state.zones.list(ZoneLocation::Hand(P1))[0];
    move_to(&mut e, ally, ZoneLocation::Graveyard(P1));
    assert_eq!(e.state.object(ally).unwrap().zone, Zone::Graveyard);
    let token = on_battlefield(&e, P2, card("Ornithopter")).unwrap();
    e.state.object_mut(token).unwrap().card = None;
    move_to(&mut e, token, ZoneLocation::Graveyard(P2));
    assert_eq!(e.state.object(token).unwrap().zone, Zone::Graveyard);
    let copy = e.state.zones.list(ZoneLocation::Hand(P2))[0];
    e.state
        .object_mut(copy)
        .unwrap()
        .riders
        .push(Rider::SpellCopy);
    move_to(&mut e, copy, ZoneLocation::Graveyard(P2));
    assert_eq!(e.state.object(copy).unwrap().zone, Zone::Graveyard);
    let dauthi = on_battlefield(&e, P0, card("Dauthi Voidwalker")).unwrap();
    e.state
        .object_mut(dauthi)
        .unwrap()
        .status
        .insert(Status::PHASED_OUT);
    let normal = e.state.zones.list(ZoneLocation::Hand(P2))[0];
    move_to(&mut e, normal, ZoneLocation::Graveyard(P2));
    assert_eq!(e.state.object(normal).unwrap().zone, Zone::Graveyard);
    e.state
        .object_mut(dauthi)
        .unwrap()
        .status
        .remove(Status::PHASED_OUT);
    let other = e.state.zones.list(ZoneLocation::Hand(P2))[0];
    move_to(&mut e, other, ZoneLocation::Graveyard(P2));
    exiled(&e, other);
}
#[test]
fn dauthi_does_not_outlive_a_destroy_all_instruction_in_the_same_resolution() {
    let mut e = setup(false);
    let source = in_hand(&e, P0, card("Toxic Deluge")).unwrap();
    let elf = on_battlefield(&e, P1, card("Llanowar Elves")).unwrap();
    let top = *e
        .state
        .zones
        .list(ZoneLocation::Library(P1))
        .last()
        .unwrap();
    let mut res = crate::resolve::Resolution {
        source,
        on_stack: source,
        controller: P0,
        effects: vec![
            Effect::DestroyAll {
                filter: &Filter::CREATURE,
                no_regen: false,
            },
            Effect::Mill {
                target: baylee_cards_dsl::PlayerRel::EachOpponent,
                amount: baylee_cards_dsl::Amount::Fixed(1),
            },
        ],
        pc: 0,
        targets: smallvec::SmallVec::new(),
        second_targets: smallvec::SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        event_mana: None,
        retarget_left: None,
    };
    assert!(matches!(
        crate::resolve::run(&mut e.state, &mut res),
        crate::resolve::Flow::Complete
    ));
    exiled(&e, elf);
    assert_eq!(e.state.object(top).unwrap().zone, Zone::Graveyard);
}
