use super::*;
use baylee_core::generated::index;

#[test]
fn public_revelation_reaches_every_seat_and_spectators_then_becomes_private_again() {
    let p0 = PlayerId::new(0);
    let mut preset = mixed_print_preset();
    preset.seats.push(preset.seats[1].clone());
    preset.seats[0].starting_battlefield.push(DeckEntry {
        card: index::REVELATION,
        print: PrintRef::new(0),
    });
    let mut engine = Engine::new(&preset, Registry).unwrap();
    settle(&mut engine, None);
    let source = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| engine.state().object(*id).unwrap().card.unwrap().index == index::REVELATION)
        .unwrap();
    for visible in [true, false] {
        for seat in (0..3).map(PlayerId::new) {
            let view = seen_by(&engine, seat);
            assert_eq!(view.shared_hands.len(), if visible { 2 } else { 0 });
            for hand in &view.shared_hands {
                assert_ne!(hand.player, seat);
                assert_eq!(
                    hand.cards.iter().map(|c| c.id).collect::<Vec<_>>(),
                    *engine.state().zones.list(ZoneLocation::Hand(hand.player))
                );
            }
            assert!(view.library_tops.is_empty());
        }
        let watcher = spectator_view(engine.state(), 1, None, SeatSet::new(), SeatSet::new(), &[]);
        assert_eq!(watcher.revealed_hands.len(), if visible { 3 } else { 0 });
        let converted = watcher.into_player_view(p0);
        assert!(
            converted.hand.is_empty(),
            "a spectator never acquires a private hand"
        );
        assert_eq!(converted.shared_hands.len(), if visible { 3 } else { 0 });
        if visible {
            engine
                .dev_state_mut(p0)
                .unwrap()
                .move_object(
                    source,
                    ZoneLocation::Graveyard(p0),
                    baylee_engine::zone::ZonePosition::Top,
                    baylee_engine::event::Cause::Effect,
                )
                .unwrap();
        }
    }
}

#[test]
fn field_of_dreams_reveals_only_one_library_card_per_player_to_every_view() {
    let mut preset = mixed_print_preset();
    preset.seats.push(preset.seats[1].clone());
    preset.seats[0].starting_battlefield.push(DeckEntry {
        card: index::FIELD_OF_DREAMS,
        print: PrintRef::new(0),
    });
    let mut engine = Engine::new(&preset, Registry).unwrap();
    settle(&mut engine, None);
    let expected: Vec<_> = (0..3)
        .map(PlayerId::new)
        .map(|p| {
            *engine
                .state()
                .zones
                .list(ZoneLocation::Library(p))
                .last()
                .unwrap()
        })
        .collect();
    for seat in (0..3).map(PlayerId::new) {
        let view = seen_by(&engine, seat);
        assert_eq!(
            view.library_tops.iter().map(|c| c.id).collect::<Vec<_>>(),
            expected
        );
        assert!(view.shared_hands.is_empty());
    }
    let watcher = spectator_view(engine.state(), 1, None, SeatSet::new(), SeatSet::new(), &[]);
    assert_eq!(
        watcher
            .library_tops
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>(),
        expected
    );
    assert!(watcher.revealed_hands.is_empty());
}
