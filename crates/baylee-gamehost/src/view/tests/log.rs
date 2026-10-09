use super::*;

// ---- the game log names an object as the view would have (#262)

use crate::log::GameLog;
use baylee_engine::event::{Cause, GameEvent};
use baylee_engine::zone::{Zone, ZonePosition};
use baylee_view::{LogEvent, LogFrom, LogObject, LogPlace, LogZone};

/// A table past its mulligans, and its log so far.
fn a_logged_table() -> (Engine<Registry>, GameLog) {
    let mut engine = Engine::new(&mixed_print_preset(), Registry).expect("game starts");
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine
            .apply(player, baylee_engine::choice::PlayerAction::MulliganKeep)
            .expect("keeps");
    }
    let log = GameLog::new(engine.state());
    (engine, log)
}

/// What `seat` is told of the lines from `from` on.
fn told_since(log: &GameLog, seat: PlayerId, from: usize) -> Vec<LogEvent> {
    log.told(seat, from, log.len())
        .into_iter()
        .map(|line| line.event)
        .collect()
}

/// The handle of a name the seat may know, if it may.
fn handle(object: &LogObject) -> Option<ObjectId> {
    match object {
        LogObject::Known { id, .. } | LogObject::FaceDown { id } => Some(*id),
        LogObject::Hidden => None,
    }
}

/// Cards drawn are named to the seat that drew them, and to every other
/// seat they are "a card": no handle, so a card cannot be followed from
/// the draw to the cast that shows it.
#[test]
fn a_draw_names_its_cards_to_the_drawer_and_to_nobody_else() {
    let (mut engine, mut log) = a_logged_table();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let from = log.len();
    let drawn = engine
        .dev_state_mut(me)
        .expect("dev commands")
        .draw_cards(me, 2);
    log.consume(engine.state());

    let [LogEvent::Drew { player, cards }] = &told_since(&log, me, from)[..] else {
        panic!("{:?}", told_since(&log, me, from))
    };
    assert_eq!(*player, me);
    assert!(
        cards
            .iter()
            .all(|card| matches!(card, LogObject::Known { card: Some(_), .. })),
        "the drawer knows what it drew"
    );
    assert_eq!(cards.iter().filter_map(handle).collect::<Vec<_>>(), drawn);
    assert_eq!(
        told_since(&log, them, from),
        [LogEvent::Drew {
            player: me,
            cards: vec![LogObject::Hidden, LogObject::Hidden]
        }]
    );
}

/// Play the actual spell: its public reveal includes lands, its chooser
/// receives only legal options, and unrelated hidden hands stay hidden.
#[test]
fn thoughtseize_reveals_only_the_target_hand_to_every_seat() {
    let entry = |name: &str| DeckEntry {
        card: baylee_cards::generated::ALL
            .iter()
            .find(|(_, card)| card.name() == name)
            .unwrap()
            .1
            .index,
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    preset.seats.push(preset.seats[1].clone());
    preset.seats[0].starting_hand = Some(vec![entry("Thoughtseize")]);
    preset.seats[0].starting_battlefield = vec![entry("Swamp")];
    preset.seats[1].starting_hand = Some(vec![entry("Island"), entry("Sol Ring")]);
    preset.seats[2].starting_hand = Some(vec![entry("Lightning Bolt")]);
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let view = settle(&mut engine, None);
    let (me, them, other) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let target_hand = engine.state().zones.list(ZoneLocation::Hand(them)).clone();
    let mut log = GameLog::new(engine.state());
    let from = log.len();
    engine
        .apply(
            me,
            PlayerAction::ActivateManaAbility {
                source: view.battlefield[0].id,
            },
        )
        .unwrap();
    let spell = view
        .hand
        .iter()
        .find(|o| o.name == "Thoughtseize")
        .unwrap()
        .id;
    engine
        .apply(me, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine.apply(me, PlayerAction::ChoosePlayer(them)).unwrap();
    for _ in 0..6 {
        if let Pending::Priority { player, .. } = engine.pending() {
            engine.apply(*player, PlayerAction::PassPriority).unwrap();
        } else {
            break;
        }
    }
    log.consume(engine.state());
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending()
    else {
        panic!("Thoughtseize must ask its controller");
    };
    assert_eq!(*player, me);
    let chosen = options[0];
    for seat in [me, them, other] {
        let events = told_since(&log, seat, from);
        let revealed: Vec<ObjectId> = events
            .iter()
            .filter_map(|event| match event {
                LogEvent::Revealed { cards, .. } => Some(cards.iter().filter_map(handle)),
                _ => None,
            })
            .flatten()
            .collect();
        assert_eq!(
            revealed, target_hand,
            "all and only the targeted hand is revealed"
        );
        let shown = seen_by(&engine, seat);
        if seat == me {
            assert_eq!(shown.looking_at.len(), 1);
            assert_eq!(shown.looking_at[0].name, "Sol Ring");
        } else {
            assert!(
                shown.looking_at.is_empty(),
                "only the caster gets the chooser"
            );
        }
        assert!(
            shown
                .hand
                .iter()
                .all(|card| engine.state().object(card.id).unwrap().owner == seat)
        );
    }
    engine
        .apply(
            me,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
    assert!(
        seen_by(&engine, me).looking_at.is_empty(),
        "temporary access ends with the choice"
    );
    assert_eq!(seen_by(&engine, me).graveyards[1][0].name, "Sol Ring");
}

/// Looking is private, unlike revealing: the third seat learns nothing,
/// and access ends immediately when the activating player acknowledges.
#[test]
fn glasses_of_urza_keeps_hand_inspection_private_and_temporary() {
    let entry = |name: &str| DeckEntry {
        card: baylee_cards::generated::ALL
            .iter()
            .find(|(_, card)| card.name() == name)
            .unwrap()
            .1
            .index,
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    preset.seats.push(preset.seats[1].clone());
    preset.seats[0].starting_hand = Some(vec![entry("Forest")]);
    preset.seats[0].starting_battlefield = vec![entry("Glasses of Urza")];
    preset.seats[1].starting_hand = Some(vec![entry("Island"), entry("Sol Ring")]);
    preset.seats[2].starting_hand = Some(vec![entry("Lightning Bolt")]);
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let view = settle(&mut engine, None);
    let (me, them, other) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let target_hand = engine.state().zones.list(ZoneLocation::Hand(them)).clone();
    let mut log = GameLog::new(engine.state());
    let from = log.len();
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: view
                    .battlefield
                    .iter()
                    .find(|o| o.name == "Glasses of Urza")
                    .unwrap()
                    .id,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            me,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them],
            },
        )
        .unwrap();
    for _ in 0..6 {
        if let Pending::Priority { player, .. } = engine.pending() {
            engine.apply(*player, PlayerAction::PassPriority).unwrap();
        } else {
            break;
        }
    }
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards {
            prompt: baylee_engine::choice::ChoicePrompt::LookAtHand,
            ..
        }
    ));
    log.consume(engine.state());
    for seat in [me, them, other] {
        assert!(
            !told_since(&log, seat, from)
                .iter()
                .any(|event| matches!(event, LogEvent::Revealed { .. })),
            "inspection must not enter the public log"
        );
        let shown = seen_by(&engine, seat);
        if seat == me {
            assert_eq!(
                shown
                    .looking_at
                    .iter()
                    .map(|card| card.id)
                    .collect::<Vec<_>>(),
                target_hand
            );
        } else {
            assert!(shown.looking_at.is_empty());
        }
    }
    engine
        .apply(me, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    for seat in [me, them, other] {
        assert!(seen_by(&engine, seat).looking_at.is_empty());
    }
    assert_eq!(
        *engine.state().zones.list(ZoneLocation::Hand(them)),
        target_hand
    );
}

/// Earlier hand selections never reveal identities or move cards before
/// the remaining players have chosen.
#[test]
#[allow(clippy::too_many_lines)] // One played three-seat privacy boundary, before and after commitment.
fn balance_keeps_hand_choices_private_until_all_players_have_chosen() {
    let entry = |name: &str| DeckEntry {
        card: baylee_cards::generated::ALL
            .iter()
            .find(|(_, card)| card.name() == name)
            .unwrap()
            .1
            .index,
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    preset.seats.push(preset.seats[1].clone());
    for seat in &mut preset.seats {
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_battlefield = vec![entry("Plains"); 2];
    preset.seats[0].starting_hand =
        Some(vec![entry("Balance"), entry("Island"), entry("Sol Ring")]);
    preset.seats[1].starting_hand = Some(vec![entry("Mountain"), entry("Lightning Bolt")]);
    preset.seats[2].starting_hand = Some(vec![entry("Forest")]);
    let mut engine = Engine::new(&preset, Registry).unwrap();
    settle(&mut engine, None);
    let (me, them, other) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let lands = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    for source in lands {
        engine
            .apply(me, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = seen_by(&engine, me)
        .hand
        .iter()
        .find(|o| o.name == "Balance")
        .unwrap()
        .id;
    engine.apply(me, PlayerAction::CastSpell { card }).unwrap();
    for _ in 0..6 {
        if let Pending::Priority { player, .. } = engine.pending() {
            engine.apply(*player, PlayerAction::PassPriority).unwrap();
        } else {
            break;
        }
    }
    let my_hand = engine.state().zones.list(ZoneLocation::Hand(me)).clone();
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(them)).clone();
    let mut log = GameLog::new(engine.state());
    let from = log.len();
    let Pending::ChooseCards {
        player, min, max, ..
    } = engine.pending()
    else {
        panic!("{:?}", engine.pending());
    };
    assert_eq!((*player, *min, *max), (me, 1, 1));
    engine
        .apply(
            me,
            PlayerAction::ChooseObjects {
                objects: vec![my_hand[0]],
            },
        )
        .unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseCards { player, .. } if *player == them));
    log.consume(engine.state());
    for seat in [me, them, other] {
        assert!(
            told_since(&log, seat, from).is_empty(),
            "private choices produce no public line"
        );
        let view = seen_by(&engine, seat);
        assert!(
            view.looking_at
                .iter()
                .all(|o| their_hand.contains(&o.id) && seat == them)
        );
        assert!(view.hand.iter().all(|o| {
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(seat))
                .contains(&o.id)
        }));
    }
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(me)), &my_hand);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(them)),
        &their_hand
    );
    engine
        .apply(
            them,
            PlayerAction::ChooseObjects {
                objects: vec![their_hand[0]],
            },
        )
        .unwrap();
    log.consume(engine.state());
    for seat in [me, them, other] {
        let lines = told_since(&log, seat, from);
        let discarded: Vec<_> = lines
            .iter()
            .filter_map(|event| {
                if let LogEvent::Discarded { card, .. } = event {
                    handle(card)
                } else {
                    None
                }
            })
            .collect();
        assert!(discarded.contains(&my_hand[1]) && discarded.contains(&their_hand[1]));
        assert!(!discarded.contains(&my_hand[0]) && !discarded.contains(&their_hand[0]));
    }
}

#[test]
fn balance_public_keep_log_preserves_face_down_identity() {
    let (mut engine, mut log) = a_logged_table();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let land = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    let from = log.len();
    let state = engine.dev_state_mut(me).unwrap();
    state
        .object_mut(land)
        .unwrap()
        .status
        .insert(baylee_engine::object::Status::FACE_DOWN);
    state.journal.record(GameEvent::CardsKept {
        player: me,
        cards: vec![land],
    });
    log.consume(engine.state());
    for seat in [me, them] {
        let Some(LogEvent::CardsKept { cards, .. }) = told_since(&log, seat, from).pop() else {
            panic!("missing keep announcement");
        };
        if seat == me {
            assert!(matches!(&cards[0], LogObject::Known { id, .. } if *id == land));
        } else {
            assert_eq!(cards, vec![LogObject::FaceDown { id: land }]);
        }
    }
}

/// A card taken from a library to a hand is the searcher's to know. Once
/// revealed on the way, as a search for anything narrower than "a card"
/// has to be, it is everyone's.
#[test]
fn a_card_found_in_a_library_is_named_to_the_table_only_once_revealed() {
    let (mut engine, mut log) = a_logged_table();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let found = library(&engine, me, 2);
    let from = log.len();
    let state = engine.dev_state_mut(me).expect("dev commands");
    state
        .move_object(
            found[0],
            ZoneLocation::Hand(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("found");
    // The engine's own order: shown from the library, then moved.
    state.journal.record(GameEvent::Revealed {
        player: me,
        cards: vec![found[1]],
    });
    state
        .move_object(
            found[1],
            ZoneLocation::Hand(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("found");
    log.consume(engine.state());

    let theirs = told_since(&log, them, from);
    assert_eq!(
        theirs[0],
        LogEvent::Moved {
            place: None,
            object: LogObject::Hidden,
            owner: me,
            from: LogZone::Library,
            to: LogZone::Hand
        }
    );
    assert!(
        matches!(&theirs[1], LogEvent::Revealed { cards, .. } if cards.iter().filter_map(handle).eq([found[1]])),
        "{:?}",
        theirs[1]
    );
    assert!(
        matches!(&theirs[2], LogEvent::Moved { object: LogObject::Known { id, card: Some(_), .. }, .. } if *id == found[1]),
        "{:?}",
        theirs[2]
    );
    assert!(
        told_since(&log, me, from).iter().all(|event| event
            .objects()
            .all(|o| matches!(o, LogObject::Known { .. }))),
        "the searcher knows both"
    );
}

/// A card put from a hand into a library (a Brainstorm's put-back) is
/// its owner's to know and nobody else's; where in the library it went
/// is everyone's (#300).
#[test]
fn a_card_put_back_into_a_library_is_its_owners_to_know() {
    let (mut engine, mut log) = a_logged_table();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let card = engine.state().zones.list(ZoneLocation::Hand(me))[0];
    let from = log.len();
    engine
        .dev_state_mut(me)
        .expect("dev commands")
        .move_object(
            card,
            ZoneLocation::Library(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("put back");
    log.consume(engine.state());

    assert!(matches!(
        &told_since(&log, me, from)[..],
        [LogEvent::Moved { object: LogObject::Known { id, .. }, place: Some(LogPlace::Top), .. }] if *id == card
    ));
    assert_eq!(
        told_since(&log, them, from),
        [LogEvent::Moved {
            place: Some(LogPlace::Top),
            object: LogObject::Hidden,
            owner: me,
            from: LogZone::Hand,
            to: LogZone::Library
        }]
    );
}

/// A land played from a hand is one line, the play's: the move that put
/// it onto the battlefield is not told again, to any seat.
#[test]
fn a_land_played_from_a_hand_is_one_line() {
    let (mut engine, mut log) = a_logged_table();
    let me = PlayerId::new(0);
    for _ in 0..100 {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("{:?}", engine.pending())
        };
        let turn = &engine.state().turn;
        if player == me && turn.active == me && turn.phase == EnginePhase::FirstMain {
            break;
        }
        engine
            .apply(player, PlayerAction::PassPriority)
            .expect("passes");
    }
    log.consume(engine.state());
    let land = engine.state().zones.list(ZoneLocation::Hand(me))[0];
    let from = log.len();
    engine
        .apply(me, PlayerAction::PlayLand { card: land })
        .expect("plays its land");
    log.consume(engine.state());

    for seat in [me, PlayerId::new(1)] {
        let told = told_since(&log, seat, from);
        assert!(
            matches!(
                &told[..],
                [LogEvent::LandPlayed { player, land: LogObject::Known { id, .. }, .. }]
                    if *player == me && *id == land
            ),
            "{told:?}"
        );
    }
}

/// A land played from a graveyard is one line too, which says where it
/// came from (#300): the move before it is not told again.
#[test]
fn a_land_played_from_a_graveyard_says_so_in_one_line() {
    let (mut engine, mut log) = a_logged_table();
    let me = PlayerId::new(0);
    let land = engine.state().zones.list(ZoneLocation::Hand(me))[0];
    let state = engine.dev_state_mut(me).expect("dev commands");
    state
        .move_object(
            land,
            ZoneLocation::Graveyard(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("milled");
    log.consume(engine.state());
    let from = log.len();
    // The engine's own order (`casting::play_land`): moved, then played.
    let state = engine.dev_state_mut(me).expect("dev commands");
    state
        .move_object(
            land,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("played");
    state.journal.record(GameEvent::LandPlayed {
        object: land,
        player: me,
    });
    log.consume(engine.state());

    let told = told_since(&log, PlayerId::new(1), from);
    assert!(
        matches!(
            &told[..],
            [LogEvent::LandPlayed {
                land: LogObject::Known { id: played, .. },
                from: Some(LogFrom { zone: LogZone::Graveyard, owner }),
                ..
            }] if *played == land && *owner == me
        ),
        "{told:?}"
    );
}

/// A spell says where it was cast from, which the log reads off the move
/// onto the stack that comes first in the same action (CR 601.2a).
#[test]
fn a_cast_says_where_the_spell_came_from() {
    let (mut engine, mut log) = a_logged_table();
    let me = PlayerId::new(0);
    let spell = engine.state().zones.list(ZoneLocation::Hand(me))[0];
    let from = log.len();
    // The engine's own order (`cast_wizard`): moved, then cast.
    let state = engine.dev_state_mut(me).expect("dev commands");
    state
        .move_object(spell, ZoneLocation::Stack, ZonePosition::Top, Cause::Spell)
        .expect("cast");
    state.journal.record(GameEvent::SpellCast {
        object: spell,
        player: me,
    });
    log.consume(engine.state());

    for seat in [me, PlayerId::new(1)] {
        let told = told_since(&log, seat, from);
        assert!(
            matches!(
                &told[..],
                [LogEvent::Cast {
                    from: Some(LogFrom { zone: LogZone::Hand, owner }),
                    ..
                }] if *owner == me
            ),
            "{told:?}"
        );
    }
}

/// Cards put into a library that is then shuffled in the same action are
/// said to be shuffled in, and the shuffle is not a line of its own. A
/// search that takes a card out and shuffles keeps its shuffle line:
/// nothing went in.
#[test]
fn a_card_shuffled_into_a_library_is_one_line() {
    let (mut engine, mut log) = a_logged_table();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let cards: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Hand(me))[..2].to_vec();
    let state = engine.dev_state_mut(me).expect("dev commands");
    for card in &cards {
        state
            .move_object(
                *card,
                ZoneLocation::Graveyard(me),
                ZonePosition::Top,
                Cause::Effect,
            )
            .expect("milled");
    }
    log.consume(engine.state());
    let from = log.len();
    let state = engine.dev_state_mut(me).expect("dev commands");
    for card in &cards {
        state
            .move_object(
                *card,
                ZoneLocation::Library(me),
                ZonePosition::Top,
                Cause::Effect,
            )
            .expect("put in");
    }
    state.journal.record(GameEvent::Shuffled {
        player: me,
        zone: Zone::Library,
    });
    log.consume(engine.state());
    let told = told_since(&log, them, from);
    assert_eq!(told.len(), 2, "{told:?}");
    assert!(
        told.iter().all(|event| matches!(
            event,
            LogEvent::Moved {
                from: LogZone::Graveyard,
                to: LogZone::Library,
                place: Some(LogPlace::Shuffled),
                ..
            }
        )),
        "{told:?}"
    );

    let from = log.len();
    let found = engine.state().zones.list(ZoneLocation::Library(me))[0];
    let state = engine.dev_state_mut(me).expect("dev commands");
    state
        .move_object(
            found,
            ZoneLocation::Hand(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("found");
    state.journal.record(GameEvent::Shuffled {
        player: me,
        zone: Zone::Library,
    });
    log.consume(engine.state());
    let told = told_since(&log, them, from);
    assert!(
        matches!(
            &told[..],
            [
                LogEvent::Moved { to: LogZone::Hand, place: None, .. },
                LogEvent::Shuffled { player },
            ] if *player == me
        ),
        "{told:?}"
    );
}

/// Every line is stamped with the time the log was last told (#300).
#[test]
fn a_line_is_stamped_with_the_time_it_was_written() {
    let (mut engine, mut log) = a_logged_table();
    let me = PlayerId::new(0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(me)).clone();
    let from = log.len();
    for (card, at) in hand[..2].iter().zip([1_000, 2_000]) {
        log.tell_time(at);
        engine
            .dev_state_mut(me)
            .expect("dev commands")
            .move_object(
                *card,
                ZoneLocation::Graveyard(me),
                ZonePosition::Top,
                Cause::Effect,
            )
            .expect("milled");
        log.consume(engine.state());
    }
    let stamps: Vec<u64> = log
        .told(me, from, log.len())
        .iter()
        .map(|line| line.at)
        .collect();
    assert_eq!(stamps, [1_000, 2_000]);
}

/// A face-down permanent is named to its controller, and to everyone else
/// is the blank the view shows them, with the same handle and no card.
#[test]
fn a_face_down_permanent_is_named_as_the_view_shows_it() {
    let (mut engine, mut log) = a_logged_table();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let land = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    let from = log.len();
    let state = engine.dev_state_mut(me).expect("dev commands");
    state
        .object_mut(land)
        .expect("the permanent is there")
        .status
        .insert(baylee_engine::object::Status::FACE_DOWN);
    state.journal.record(GameEvent::CounterChanged {
        object: land,
        kind: baylee_cards_dsl::CounterKind::Charge,
        old: 0,
        new: 1,
    });
    log.consume(engine.state());

    let named = |seat| match told_since(&log, seat, from).pop() {
        Some(LogEvent::Counters { object, .. }) => object,
        other => panic!("{other:?}"),
    };
    assert!(matches!(named(me), LogObject::Known { id, card: Some(_), .. } if id == land));
    assert_eq!(named(them), LogObject::FaceDown { id: land });
    let shown = player_view(engine.state(), them, 1, None, &SeatContext::default(), &[])
        .battlefield
        .into_iter()
        .find(|o| o.id == land)
        .expect("on the battlefield");
    assert!(shown.card.is_none(), "and the view agrees");
}

/// Face down in exile, the same: the view shows the other seats a blank
/// with a handle, and the log names it the same way.
#[test]
fn a_face_down_card_in_exile_is_named_as_the_view_shows_it() {
    let (mut engine, mut log) = a_logged_table();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let card = engine.state().zones.list(ZoneLocation::Hand(me))[0];
    let from = log.len();
    let state = engine.dev_state_mut(me).expect("dev commands");
    state
        .move_object(
            card,
            ZoneLocation::Exile(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("exiled");
    state
        .object_mut(card)
        .expect("in exile")
        .status
        .insert(baylee_engine::object::Status::FACE_DOWN);
    log.consume(engine.state());

    assert_eq!(
        told_since(&log, them, from),
        [LogEvent::Moved {
            place: None,
            object: LogObject::FaceDown { id: card },
            owner: me,
            from: LogZone::Hand,
            to: LogZone::Exile
        }]
    );
    assert!(matches!(
        &told_since(&log, me, from)[..],
        [LogEvent::Moved {
            object: LogObject::Known { card: Some(_), .. },
            ..
        }]
    ));
    let shown = player_view(engine.state(), them, 1, None, &SeatContext::default(), &[])
        .exile
        .concat()
        .into_iter()
        .find(|o| o.id == card)
        .expect("the view shows the exiled card");
    assert!(shown.card.is_none(), "and the view agrees");
}

/// A permanent no line has named yet is still named as every seat saw
/// it once it is gone: the log remembers what stood in a public zone.
#[test]
fn a_permanent_gone_before_a_line_named_it_is_named_as_it_stood() {
    let (mut engine, mut log) = a_logged_table();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    // Laid out by the preset, which is no line of the log's.
    let land = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    let from = log.len();
    let state = engine.dev_state_mut(me).expect("dev commands");
    state
        .zones
        .list_mut(ZoneLocation::Battlefield)
        .retain(|id| *id != land);
    state.arena.remove(land).expect("it was there");
    state.journal.record(GameEvent::DamageDealt {
        source: Some(land),
        target: baylee_engine::event::DamageTarget::Player(them),
        amount: 1,
        is_combat: false,
    });
    log.consume(engine.state());

    assert!(
        matches!(
            &told_since(&log, them, from)[..],
            [LogEvent::Damage { source: Some(LogObject::Known { id, name, .. }), .. }]
                if *id == land && name == "Island"
        ),
        "{:?}",
        told_since(&log, them, from)
    );
}

/// The monarch's abilities have no source (CR 724.2): there is nothing
/// to name one by, and no line says an unknown object did something.
#[test]
fn a_trigger_with_no_source_gets_no_line() {
    let (mut engine, mut log) = a_logged_table();
    let me = PlayerId::new(0);
    let from = log.len();
    engine
        .dev_state_mut(me)
        .expect("dev commands")
        .journal
        .record(GameEvent::AbilityTriggered {
            object: ObjectId::new(9_999, 0),
            source: ObjectId::NO_SOURCE,
            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
            controller: me,
        });
    log.consume(engine.state());
    assert_eq!(told_since(&log, me, from), []);
}

/// The monarch at a table of four (CR 724): every seat's view names who
/// holds the crown, and every seat is told the line that says it moved,
/// once per move. A monarch told to become the monarch again is no line.
#[test]
fn every_seat_sees_the_monarch_and_is_told_when_the_crown_moves() {
    let mut preset = mixed_print_preset();
    let extra = preset.seats[1].clone();
    preset.seats.push(extra.clone());
    preset.seats.push(extra);
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine
            .apply(player, baylee_engine::choice::PlayerAction::MulliganKeep)
            .expect("keeps");
    }
    let mut log = GameLog::new(engine.state());
    let seats: Vec<PlayerId> = (0..4).map(PlayerId::new).collect();
    let views = |engine: &Engine<Registry>| -> Vec<Option<PlayerId>> {
        seats
            .iter()
            .map(|&s| player_view(engine.state(), s, 0, None, &SeatContext::default(), &[]).monarch)
            .collect()
    };
    assert_eq!(
        views(&engine),
        [None; 4],
        "no monarch until an effect makes one (CR 724.1)"
    );

    let from = log.len();
    for crowned in [2, 2, 3] {
        engine
            .dev_state_mut(seats[0])
            .expect("dev commands")
            .set_monarch(seats[crowned]);
    }
    log.consume(engine.state());
    assert_eq!(views(&engine), [Some(seats[3]); 4]);
    for &seat in &seats {
        assert_eq!(
            told_since(&log, seat, from),
            [
                LogEvent::BecameMonarch { player: seats[2] },
                LogEvent::BecameMonarch { player: seats[3] },
            ],
            "seat {seat:?}"
        );
    }
}

/// Library of Leng: a discarded card put on top of its owner's library
/// instead of into the graveyard is not revealed (CR 701.9c). The discard
/// line names it to its discarder and to nobody else, while a card the same
/// discard sent to the graveyard is named to every seat.
#[test]
fn a_discard_put_on_top_of_a_library_is_named_to_its_discarder_only() {
    let (mut engine, mut log) = a_logged_table();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let hand = engine.state().zones.list(ZoneLocation::Hand(me)).clone();
    let (to_top, to_graveyard) = (hand[0], hand[1]);
    let from = log.len();
    let state = engine.dev_state_mut(me).expect("dev commands");
    // The engine's own order (`GameState::discard_card`): the discard,
    // then the move.
    state.journal.record(GameEvent::Discarded {
        object: to_top,
        player: me,
    });
    state
        .move_object(
            to_top,
            ZoneLocation::Library(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("on top");
    state.journal.record(GameEvent::Discarded {
        object: to_graveyard,
        player: me,
    });
    state
        .move_object(
            to_graveyard,
            ZoneLocation::Graveyard(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("discarded");
    log.consume(engine.state());

    let discards = |seat: PlayerId| -> Vec<Option<ObjectId>> {
        told_since(&log, seat, from)
            .iter()
            .filter_map(|event| match event {
                LogEvent::Discarded { card, .. } => Some(handle(card)),
                _ => None,
            })
            .collect()
    };
    assert_eq!(discards(me), vec![Some(to_top), Some(to_graveyard)]);
    assert_eq!(
        discards(them),
        vec![None, Some(to_graveyard)],
        "the card on top of the library is a card, and only that"
    );
}
