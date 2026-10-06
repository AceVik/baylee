use super::*;

#[test]
fn target_explanation_names_the_cast_and_is_only_sent_to_its_chooser() {
    let entry = |name: &str| DeckEntry {
        card: baylee_cards::generated::ALL
            .iter()
            .find(|(_, c)| c.name() == name)
            .unwrap()
            .1
            .index,
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_hand = Some(vec![entry("Swords to Plowshares")]);
    preset.seats[0].starting_battlefield = vec![entry("Plains")];
    preset.seats[1].starting_battlefield = vec![entry("Ondu Cleric")];
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let view = settle(&mut engine, None);
    let me = PlayerId::new(0);
    engine
        .apply(
            me,
            PlayerAction::ActivateManaAbility {
                source: view.battlefield[0].id,
            },
        )
        .unwrap();
    let spell = view.hand[0].id;
    engine
        .apply(me, PlayerAction::CastSpell { card: spell })
        .unwrap();
    let context = targeting_context(&engine, me).expect("target explanation");
    assert_eq!(context.source.id, spell);
    assert_eq!(context.source.name, "Swords to Plowshares");
    assert!(
        context.whole_spell,
        "all sentences of the spell explain the effect"
    );
    assert_eq!(
        context.batch_count, 0,
        "a cast is never an identical-trigger series"
    );
    assert!(!context.second);
    assert!(
        targeting_context(&engine, PlayerId::new(1)).is_none(),
        "no private casting choice crosses seats"
    );
}

#[test]
fn maximum_hand_size_status_is_public_and_expires_with_its_source() {
    use baylee_engine::{event::Cause, zone::ZonePosition};
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![DeckEntry {
        card: baylee_cards::decks::by_name("Reliquary Tower").unwrap(),
        print: PrintRef::new(0),
    }];
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let initial = settle(&mut engine, None);
    let tower = initial.battlefield[0].id;
    let me = PlayerId::new(0);
    for seat in 0..2 {
        let view = seen_by(&engine, PlayerId::new(seat));
        assert!(view.seats[0].no_max_hand_size);
        assert!(!view.seats[1].no_max_hand_size);
    }
    engine
        .dev_state_mut(me)
        .unwrap()
        .move_object(
            tower,
            ZoneLocation::Graveyard(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    // A dev move bypasses the normal action boundary. Let the engine
    // synchronize continuous effects before publishing another view.
    engine
        .apply(me, baylee_engine::choice::PlayerAction::PassPriority)
        .unwrap();
    for seat in 0..2 {
        let view = seen_by(&engine, PlayerId::new(seat));
        assert!(view.seats.iter().all(|s| !s.no_max_hand_size));
    }
}

#[test]
fn courser_reveals_only_the_top_to_all_seats_and_hides_the_next_during_shock_choice() {
    use baylee_engine::{choice::PlayerAction, event::Cause, zone::ZonePosition};
    let entry = |name| DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    preset.seats.push(preset.seats[1].clone());
    preset.seats[0].starting_battlefield = vec![entry("Courser of Kruphix")];
    preset.seats[0].starting_hand = Some(vec![entry("Breeding Pool")]);
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let view = settle(&mut engine, None);
    let me = PlayerId::new(0);
    let shock = view
        .hand
        .iter()
        .find(|o| o.card.index == entry("Breeding Pool").card)
        .unwrap()
        .id;
    engine
        .dev_state_mut(me)
        .unwrap()
        .move_object(
            shock,
            ZoneLocation::Library(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();
    let hidden = engine
        .state()
        .zones
        .list(ZoneLocation::Library(me))
        .iter()
        .copied()
        .filter(|id| *id != shock)
        .collect::<Vec<_>>();
    for seat in 0..3 {
        let view = seen_by(&engine, PlayerId::new(seat));
        assert_eq!(view.library_tops.len(), 1);
        assert_eq!(view.library_tops[0].id, shock);
        assert!(view.cards().any(|c| c == entry("Breeding Pool").card));
        for id in &hidden {
            assert!(view.object(*id).is_none());
        }
    }
    engine
        .apply(me, PlayerAction::PlayLand { card: shock })
        .unwrap();
    for seat in 0..3 {
        assert!(
            seen_by(&engine, PlayerId::new(seat))
                .library_tops
                .is_empty()
        );
    }
    engine.apply(me, PlayerAction::YesNo(false)).unwrap();
    for seat in 0..3 {
        let view = seen_by(&engine, PlayerId::new(seat));
        assert_eq!(view.library_tops.len(), 1);
        assert_ne!(view.library_tops[0].id, shock);
    }
    let courser = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine.state().object(*id).unwrap().card.unwrap().index
                == entry("Courser of Kruphix").card
        })
        .unwrap();
    engine
        .dev_state_mut(me)
        .unwrap()
        .move_object(
            courser,
            ZoneLocation::Graveyard(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    for seat in 0..3 {
        assert!(
            seen_by(&engine, PlayerId::new(seat))
                .library_tops
                .is_empty()
        );
    }
}

#[test]
fn suspended_cards_and_their_countdowns_are_public_to_every_seat() {
    let vision = baylee_cards::decks::by_name("Ancestral Vision").unwrap();
    let mut preset = mixed_print_preset();
    preset.seats.push(preset.seats[1].clone());
    preset.seats[0].starting_hand = Some(vec![DeckEntry {
        card: vision,
        print: PrintRef::new(0),
    }]);
    preset.seats[0].starting_battlefield = vec![DeckEntry {
        card: island(),
        print: PrintRef::new(0),
    }];
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let view = settle(&mut engine, None);
    let me = PlayerId::new(0);
    let card = view
        .hand
        .iter()
        .find(|o| o.card.index == vision)
        .unwrap()
        .id;
    engine
        .apply(
            me,
            PlayerAction::ActivateManaAbility {
                source: view.battlefield[0].id,
            },
        )
        .unwrap();
    engine.apply(me, PlayerAction::Suspend { card }).unwrap();
    for seat in 0..3 {
        let view = seen_by(&engine, PlayerId::new(seat));
        let shown = view.exile[0].iter().find(|o| o.id == card).unwrap();
        assert!(shown.suspended);
        assert_eq!(shown.name, "Ancestral Vision");
        assert!(
            shown
                .counters
                .iter()
                .any(|c| c.kind == CounterKind::Time && c.count == 4)
        );
        assert!(view.battlefield.iter().all(|o| !o.suspended));
    }
}

#[test]
fn reflections_chosen_type_reaches_every_seat_after_casting() {
    let reflections = baylee_cards::decks::by_name("Reflections of Littjara").unwrap();
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_hand = Some(vec![DeckEntry {
        card: reflections,
        print: PrintRef::new(0),
    }]);
    preset.seats[0].starting_battlefield = vec![
        DeckEntry {
            card: island(),
            print: PrintRef::new(0)
        };
        5
    ];
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let view = settle(&mut engine, None);
    let me = PlayerId::new(0);
    for object in &view.battlefield {
        engine
            .apply(me, PlayerAction::ActivateManaAbility { source: object.id })
            .unwrap();
    }
    let card = view
        .hand
        .iter()
        .find(|o| o.card.index == reflections)
        .unwrap()
        .id;
    engine.apply(me, PlayerAction::CastSpell { card }).unwrap();
    for _ in 0..5 {
        if let Pending::Priority { player, .. } = engine.pending() {
            engine.apply(*player, PlayerAction::PassPriority).unwrap();
        } else {
            break;
        }
    }
    assert!(matches!(engine.pending(), Pending::ChooseSubtype { .. }));
    let ally = baylee_core::generated::subtypes::creature::ALLY;
    engine.apply(me, PlayerAction::ChooseSubtype(ally)).unwrap();
    for seat in [me, PlayerId::new(1)] {
        let view = seen_by(&engine, seat);
        assert_eq!(view.object(card).unwrap().chosen_subtype, Some(ally));
        assert!(
            view.battlefield
                .iter()
                .filter(|o| o.id != card)
                .all(|o| o.chosen_subtype.is_none())
        );
    }
}

/// The card name chosen for a Pithing Needle is announced as it is chosen
/// (CR 201.4), so every seat is told it, on that permanent and no other.
#[test]
fn a_needle_s_chosen_name_reaches_every_seat() {
    let needle = baylee_cards::decks::by_name("Pithing Needle").unwrap();
    let named = baylee_cards::decks::by_name("Karn, the Great Creator").unwrap();
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_hand = Some(vec![DeckEntry {
        card: needle,
        print: PrintRef::new(0),
    }]);
    preset.seats[0].starting_battlefield = vec![DeckEntry {
        card: island(),
        print: PrintRef::new(0),
    }];
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let view = settle(&mut engine, None);
    let me = PlayerId::new(0);
    engine
        .apply(
            me,
            PlayerAction::ActivateManaAbility {
                source: view.battlefield[0].id,
            },
        )
        .unwrap();
    let card = view
        .hand
        .iter()
        .find(|o| o.card.index == needle)
        .unwrap()
        .id;
    engine.apply(me, PlayerAction::CastSpell { card }).unwrap();
    for _ in 0..5 {
        if let Pending::Priority { player, .. } = engine.pending() {
            engine.apply(*player, PlayerAction::PassPriority).unwrap();
        } else {
            break;
        }
    }
    assert!(matches!(engine.pending(), Pending::ChooseCardName { .. }));
    engine
        .apply(
            me,
            PlayerAction::ChooseCardName {
                card: named,
                face: 0,
            },
        )
        .unwrap();
    for seat in [me, PlayerId::new(1)] {
        let view = seen_by(&engine, seat);
        assert_eq!(
            view.object(card).unwrap().chosen_name,
            Some(baylee_view::NamedFace {
                card: named,
                face: 0
            }),
            "seat {seat:?} is told the name"
        );
        assert!(
            view.battlefield
                .iter()
                .filter(|o| o.id != card)
                .all(|o| o.chosen_name.is_none())
        );
    }
}

#[test]
fn black_vise_announces_its_chosen_opponent_to_every_seat() {
    let vise = baylee_cards::decks::by_name("Black Vise").unwrap();
    let mut preset = mixed_print_preset();
    preset.seats.push(preset.seats[1].clone());
    preset.seats[0].starting_hand = Some(vec![DeckEntry {
        card: vise,
        print: PrintRef::new(0),
    }]);
    preset.seats[0].starting_battlefield = vec![DeckEntry {
        card: island(),
        print: PrintRef::new(0),
    }];
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let view = settle(&mut engine, None);
    let me = PlayerId::new(0);
    engine
        .apply(
            me,
            PlayerAction::ActivateManaAbility {
                source: view.battlefield[0].id,
            },
        )
        .unwrap();
    let card = view.hand.iter().find(|o| o.card.index == vise).unwrap().id;
    engine.apply(me, PlayerAction::CastSpell { card }).unwrap();
    for _ in 0..6 {
        if let Pending::Priority { player, .. } = engine.pending() {
            engine.apply(*player, PlayerAction::PassPriority).unwrap();
        } else {
            break;
        }
    }
    let chosen = PlayerId::new(2);
    engine
        .apply(me, PlayerAction::ChoosePlayer(chosen))
        .unwrap();
    for seat in [me, PlayerId::new(1), chosen] {
        let view = seen_by(&engine, seat);
        assert_eq!(view.object(card).unwrap().chosen_opponent, Some(chosen));
        assert!(
            view.battlefield
                .iter()
                .filter(|o| o.id != card)
                .all(|o| o.chosen_opponent.is_none())
        );
    }
}

/// A Room's doors are designations (CR 709.5c), as public as the
/// permanent: every seat is told which halves are unlocked, on that
/// permanent alone. Put onto the battlefield uncast, neither is
/// (CR 709.5d); its controller unlocks the left one, and both seats see
/// it open.
#[test]
fn a_rooms_doors_reach_every_seat() {
    let room = baylee_cards::decks::by_name("Walk-In Closet").unwrap();
    let entry = |card| DeckEntry {
        card,
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![
        entry(room),
        entry(forest()),
        entry(forest()),
        entry(forest()),
    ];
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let view = settle(&mut engine, None);
    let me = PlayerId::new(0);
    let id = view
        .battlefield
        .iter()
        .find(|o| o.card.is_some_and(|c| c.index == room))
        .unwrap()
        .id;
    for seat in [me, PlayerId::new(1)] {
        assert_eq!(
            seen_by(&engine, seat).object(id).unwrap().unlocked_doors,
            Some([false, false]),
            "uncast, seat {seat:?} is told both doors are locked"
        );
    }
    for land in view
        .battlefield
        .iter()
        .filter(|o| o.card.is_some_and(|c| c.index == forest()))
    {
        engine
            .apply(me, PlayerAction::ActivateManaAbility { source: land.id })
            .unwrap();
    }
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: id,
                ability_index: baylee_engine::choice::unlock_door(0),
            },
        )
        .unwrap();
    for seat in [me, PlayerId::new(1)] {
        let view = seen_by(&engine, seat);
        assert_eq!(
            view.object(id).unwrap().unlocked_doors,
            Some([true, false]),
            "seat {seat:?} is told the left door is open"
        );
        assert!(
            view.battlefield
                .iter()
                .filter(|o| o.id != id)
                .all(|o| o.unlocked_doors.is_none()),
            "and nothing else is a Room"
        );
    }
}

/// A preset with a Teferi, Time Raveler standing on seat 1's battlefield.
fn a_table_under_teferi() -> GamePreset {
    let mut preset = mixed_print_preset();
    preset.seats[1].starting_battlefield = vec![DeckEntry {
        card: teferi_time_raveler(),
        print: PrintRef::new(0),
    }];
    preset
}

/// The seat Teferi is holding to sorcery speed is told so, and told by
/// which card.
///
/// This is the one timing fact a client cannot work out for itself. Its
/// own rule is written to err in the direction that costs the player
/// nothing — offer a spell the engine then refuses, rather than hide one
/// it would have allowed — and for this effect it erred the expensive way
/// round: an instant was offered unconditionally, the tap ran, the lands
/// were spent, and only then was the spell refused.
///
/// The object and not a flag, because the answer a player is owed is
/// *which card*, and the bystander is the seat across the table: Teferi's
/// own controller casts at whatever speed they like (CR 613 — the static
/// says "each opponent"), so a view that named it for both seats would be
/// a lock nobody could ever be outside of.
#[test]
fn the_seat_teferi_locks_is_told_which_card_is_locking_it() {
    use baylee_engine::choice::{Pending, PlayerAction};

    let preset = a_table_under_teferi();
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    // Past the mulligans, because a static is registered by a pass of the
    // machine and not by dealing the cards: the effect table is empty
    // until the game has actually started.
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan")
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    let locked = player_view(
        engine.state(),
        PlayerId::new(0),
        0,
        None,
        &SeatContext::default(),
        &[],
    );
    let theirs = player_view(
        engine.state(),
        PlayerId::new(1),
        0,
        None,
        &SeatContext::default(),
        &[],
    );

    let teferi = theirs
        .battlefield
        .iter()
        .find(|o| o.controller == PlayerId::new(1))
        .expect("their walker is on the table");

    assert_eq!(
        locked.sorcery_lock,
        Some(teferi.id),
        "the seat it holds is told which permanent holds it"
    );
    assert_eq!(
        theirs.sorcery_lock, None,
        "and its own controller is not held by it"
    );
    assert!(
        !locked.sorceries_have_flash && !theirs.sorceries_have_flash,
        "the static is the lock, not the +1: nobody's sorceries have flash"
    );
}
