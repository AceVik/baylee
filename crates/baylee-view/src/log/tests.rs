use super::*;
use baylee_core::ids::CardIndex;

fn known(n: u16) -> LogObject {
    LogObject::Known {
        id: ObjectId::new(n.into(), 0),
        card: Some(CardIdentity {
            index: CardIndex::new(u32::from(n) + 1000),
            print: PrintRef::new(n),
            face: 0,
        }),
        token: None,
        name: format!("card {n}"),
    }
}

const fn player() -> PlayerId {
    PlayerId::new(1)
}

/// Every kind of line that names objects, with the objects it names in order.
fn naming() -> Vec<(LogEvent, Vec<u16>)> {
    vec![
        (
            LogEvent::LandPlayed {
                player: player(),
                land: known(1),
                from: None,
            },
            vec![1],
        ),
        (
            LogEvent::Cast {
                player: player(),
                spell: known(1),
                from: None,
            },
            vec![1],
        ),
        (
            LogEvent::Ability {
                controller: player(),
                source: known(1),
                ability: None,
                activated: None,
            },
            vec![1],
        ),
        (LogEvent::Countered { spell: known(1) }, vec![1]),
        (
            LogEvent::DidNotResolve {
                object: known(1),
                ability: false,
            },
            vec![1],
        ),
        (
            LogEvent::Drew {
                player: player(),
                cards: vec![known(1), LogObject::Hidden, known(2)],
            },
            vec![1, 0, 2],
        ),
        (
            LogEvent::Discarded {
                player: player(),
                card: known(1),
            },
            vec![1],
        ),
        (
            LogEvent::Moved {
                object: known(1),
                owner: player(),
                from: LogZone::Hand,
                to: LogZone::Exile,
                place: None,
            },
            vec![1],
        ),
        (
            LogEvent::Created {
                object: known(1),
                controller: player(),
            },
            vec![1],
        ),
        (
            LogEvent::Damage {
                source: Some(known(1)),
                target: LogTarget::Object(known(2)),
                amount: 3,
                combat: true,
            },
            vec![1, 2],
        ),
        (
            LogEvent::Damage {
                source: None,
                target: LogTarget::Player(player()),
                amount: 3,
                combat: false,
            },
            vec![],
        ),
        (
            LogEvent::Counters {
                object: known(1),
                kind: CounterKind::Loyalty,
                old: 0,
                new: 1,
            },
            vec![1],
        ),
        (
            LogEvent::Attacked {
                attacker: known(1),
                defending: Defender::Player(player()),
            },
            vec![1],
        ),
        (
            LogEvent::Banded {
                attacker: known(1),
                with: known(2),
            },
            vec![1, 2],
        ),
        (
            LogEvent::Blocked {
                blocker: known(1),
                attacker: known(2),
            },
            vec![1, 2],
        ),
        (
            LogEvent::ControlChanged {
                object: known(1),
                old: player(),
                new: PlayerId::new(0),
            },
            vec![1],
        ),
        (LogEvent::TurnedFaceUp { object: known(1) }, vec![1]),
        (LogEvent::Transformed { object: known(1) }, vec![1]),
        (
            LogEvent::Revealed {
                player: player(),
                cards: vec![known(1), known(2)],
            },
            vec![1, 2],
        ),
        (
            LogEvent::CardsKept {
                player: player(),
                cards: vec![known(1), known(2)],
            },
            vec![1, 2],
        ),
        (
            LogEvent::Voted {
                player: player(),
                object: known(1),
            },
            vec![1],
        ),
        (
            LogEvent::Aimed {
                controller: player(),
                source: known(1),
                ability: false,
                rules: None,
                modes: 0,
                targets: vec![
                    LogTarget::Player(player()),
                    LogTarget::Object(known(2)),
                    LogTarget::Object(known(3)),
                ],
            },
            vec![1, 2, 3],
        ),
        (
            LogEvent::ClassLevel {
                object: known(1),
                level: 2,
            },
            vec![1],
        ),
        (
            LogEvent::VotedOption {
                player: player(),
                source: known(1),
                option: 0,
            },
            vec![1],
        ),
        (
            LogEvent::Life {
                player: player(),
                old: 20,
                new: 17,
            },
            vec![],
        ),
        (LogEvent::Shuffled { player: player() }, vec![]),
        (LogEvent::LoopDetected { broken: true }, vec![]),
    ]
}

const fn number(object: &LogObject) -> u16 {
    match object {
        LogObject::Known {
            card: Some(card), ..
        } => card.print.get(),
        _ => 0,
    }
}

const fn line(event: LogEvent) -> LogEntry {
    LogEntry {
        turn: 3,
        repeat: 1,
        at: 0,
        event,
        phase: None,
        step: None,
    }
}

#[test]
fn a_line_names_every_object_in_it_once_and_in_order() {
    for (event, expected) in naming() {
        let named: Vec<_> = event.objects().map(number).collect();
        assert_eq!(named, expected, "{event:?}");
    }
}

#[test]
fn hiding_every_object_of_a_line_leaves_nothing_of_them_in_it() {
    // What a host does to a line before a seat that may not know: whatever
    // `objects` finds, `objects_mut` must reach, or a name stays behind.
    for (event, expected) in naming() {
        let mut hidden = event.clone();
        let mut reached = 0;
        for object in hidden.objects_mut() {
            *object = LogObject::Hidden;
            reached += 1;
        }
        assert_eq!(reached, expected.len(), "{event:?}");
        let text = serde_json::to_string(&hidden).unwrap();
        assert!(!text.contains("card "), "{text}");
        assert!(!text.contains("print"), "{text}");
    }
}

#[test]
fn the_two_walks_over_a_line_go_in_the_same_order() {
    for (event, _) in naming() {
        let read: Vec<_> = event.objects().cloned().collect();
        let mut written = event.clone();
        let written: Vec<_> = written.objects_mut().map(|object| object.clone()).collect();
        assert_eq!(read, written, "{event:?}");
    }
}

#[test]
fn a_tail_earns_the_printings_its_known_cards_have_and_no_others() {
    let face_down = LogObject::FaceDown {
        id: ObjectId::new(4, 0),
    };
    let token = LogObject::Known {
        id: ObjectId::new(5, 0),
        card: None,
        token: Some(6),
        name: "Soldier".to_owned(),
    };
    let tail = LogTail {
        from: 9,
        entries: vec![
            line(LogEvent::Drew {
                player: player(),
                cards: vec![known(1), LogObject::Hidden, face_down, token],
            }),
            line(LogEvent::Shuffled { player: player() }),
            line(LogEvent::Blocked {
                blocker: known(2),
                attacker: known(3),
            }),
        ],
    };
    let prints: Vec<_> = tail.prints().map(PrintRef::get).collect();
    assert_eq!(prints, [1, 2, 3]);
    assert_eq!(LogTail::default().prints().count(), 0);
}

#[test]
fn a_hidden_object_carries_no_handle_and_a_face_down_one_no_name() {
    // CR 708.5: the seat may point at a face-down permanent, never at a card
    // in a hand or library.
    let text = serde_json::to_string(&LogObject::Hidden).unwrap();
    assert_eq!(text, r#""Hidden""#);
    let text = serde_json::to_value(LogObject::FaceDown {
        id: ObjectId::new(4, 0),
    })
    .unwrap();
    let fields = text["FaceDown"].as_object().unwrap();
    assert_eq!(fields.keys().collect::<Vec<_>>(), ["id"]);
}

#[test]
fn every_kind_of_line_reads_back_whole() {
    for (event, _) in naming() {
        let entry = LogEntry {
            phase: Some(Phase::Combat),
            step: Some(Step::CombatDamage),
            ..line(event)
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert_eq!(serde_json::from_str::<LogEntry>(&json).unwrap(), entry);
        let older = serde_json::to_string(&LogEntry {
            phase: None,
            step: None,
            ..entry.clone()
        })
        .unwrap();
        // A host that never said leaves both out, and a reader takes that.
        assert!(!older.contains("phase"), "{older}");
        let read: LogEntry = serde_json::from_str(&older).unwrap();
        assert_eq!((read.phase, read.step), (None, None));
    }
}
