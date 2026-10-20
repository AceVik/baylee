use super::*;
use crate::test_support::{ViewBuilder, printed, statics};
use baylee_core::ids::{PrintRef, SeatSet};
use baylee_view::{LossCause, SeatIdentity};

const ME: PlayerId = PlayerId::new(0);
const BO: PlayerId = PlayerId::new(1);
const BOLT: CardIndex = CardIndex::new(7);

fn entry(turn: u32, event: LogEvent) -> LogEntry {
    LogEntry {
        at: 0,
        turn,
        repeat: 1,
        event,
        phase: None,
    }
}

/// `n` entries that all differ, numbered from `first`.
fn entries(first: u32, n: u32) -> Vec<LogEntry> {
    (first..first + n)
        .map(|turn| entry(turn, LogEvent::TurnStarted { active: ME }))
        .collect()
}

fn tail(from: u32, entries: &[LogEntry]) -> LogTail {
    LogTail {
        from,
        entries: entries.to_vec(),
    }
}

fn view() -> PlayerView {
    ViewBuilder::new(2).build()
}

fn book(entries: &[LogEntry]) -> LogBook {
    let mut book = LogBook::new();
    assert_eq!(book.append(&tail(0, entries), &view()), entries.len());
    book
}

/// Seat 0 is Ada, who reads; seat 1 is Bo; seat 2 is Cy.
fn roster() -> GameStatic {
    let mut roster = statics(0);
    roster.seats = ["Ada", "Bo", "Cy"]
        .iter()
        .zip(0..)
        .map(|(name, seat)| SeatIdentity {
            player: PlayerId::new(seat),
            display_name: (*name).to_string(),
            is_ai: false,
            away: false,
            team: None,
        })
        .collect();
    roster
}

fn card_text(name: &str, english: &str) -> CardText {
    CardText {
        lang: String::new(),
        name: name.to_string(),
        type_line: String::new(),
        oracle_text: String::new(),
        mana_cost: String::new(),
        english_name: english.to_string(),
    }
}

/// The catalog's text for Lightning Bolt, in the language it serves.
struct Catalog(Lang);

impl CardTextLookup for Catalog {
    fn text(&self, card: CardIndex, face: u8) -> Option<CardText> {
        let name = match self.0 {
            Lang::En => "Lightning Bolt",
            Lang::De => "Blitzschlag",
        };
        (card == BOLT && face == 0).then(|| card_text(name, "Lightning Bolt"))
    }
}

fn no_text(_: CardIndex, _: u8) -> Option<CardText> {
    None
}

fn bolt(slot: u32) -> LogObject {
    LogObject::Known {
        token: None,
        id: ObjectId::new(slot, 0),
        card: Some(CardIdentity {
            index: BOLT,
            print: PrintRef::new(0),
            face: 0,
        }),
        name: "Lightning Bolt".to_string(),
    }
}

fn token(slot: u32, name: &str) -> LogObject {
    LogObject::Known {
        token: None,
        id: ObjectId::new(slot, 0),
        card: None,
        name: name.to_string(),
    }
}

/// Line 0 of `book`, as Ada reads it in `lang` with the catalog's text.
fn read(book: &LogBook, lang: Lang) -> LogLine {
    let roster = roster();
    let catalog = Catalog(lang);
    let wording = Wording {
        lang,
        seat: ME,
        statics: Some(&roster),
        texts: &catalog,
    };
    book.line(0, &wording).expect("the book holds line 0")
}

/// `event` as Ada reads it in `lang`.
fn say(lang: Lang, event: LogEvent) -> String {
    read(&book(&[entry(1, event)]), lang).text
}

/// The line crowning a monarch is marked so a panel can wear the crown,
/// and no other line is.
#[test]
fn only_the_monarchs_line_is_crowned() {
    let crowned = |event| read(&book(&[entry(1, event)]), Lang::En).crowned;
    assert!(crowned(LogEvent::BecameMonarch { player: ME }));
    assert!(!crowned(LogEvent::Shuffled { player: ME }));
}

/// A coin flip reads as the sentence it is (CR 705), for the flipper
/// and for the other seat, and says which way it fell.
#[test]
fn a_coin_flip_reads_as_won_or_lost() {
    let flip = |player, won| say(Lang::En, LogEvent::CoinFlipped { player, won });
    assert_eq!(flip(ME, true), "You flipped a coin and won the flip");
    assert_eq!(flip(ME, false), "You flipped a coin and lost the flip");
    assert_eq!(flip(BO, true), "Bo flipped a coin and won the flip");
    assert_eq!(flip(BO, false), "Bo flipped a coin and lost the flip");
}

// ---- the book ----------------------------------------------------------

/// Tails that follow each other are appended in order, each once.
#[test]
fn tails_that_follow_each_other_are_appended_in_order() {
    let all = entries(1, 5);
    let mut book = LogBook::new();
    assert_eq!(book.append(&tail(0, &all[..3]), &view()), 3);
    assert_eq!(book.append(&tail(3, &all[3..]), &view()), 2);
    assert_eq!(book.entries(), &all[..]);
    assert_eq!((book.gaps(), book.rewrites()), (0, 0));
}

/// A chunk sent twice, as several frames with one `seq` may repeat it,
/// adds nothing the second time.
#[test]
fn a_chunk_sent_twice_adds_nothing() {
    let all = entries(1, 5);
    let mut book = book(&all[..3]);
    assert_eq!(book.append(&tail(3, &all[3..]), &view()), 2);
    assert_eq!(book.append(&tail(3, &all[3..]), &view()), 0);
    assert_eq!(book.append(&tail(1, &all[1..4]), &view()), 0);
    assert_eq!(book.entries(), &all[..]);
}

/// A snapshot tells the log from its start: the book keeps what it holds
/// and adds only the lines it was missing, each exactly once.
#[test]
fn a_snapshot_from_the_start_adds_only_what_was_missing() {
    let all = entries(1, 6);
    let mut book = book(&all[..2]);
    assert_eq!(book.append(&tail(0, &all), &view()), 4);
    assert_eq!(book.entries(), &all[..]);
    // And a reconnect tells it all again.
    assert_eq!(book.append(&tail(0, &all), &view()), 0);
    assert_eq!(book.entries(), &all[..]);
    assert_eq!(book.rewrites(), 0);
}

/// An empty tail changes nothing, although a question asked again
/// carries one that says it starts at 0.
#[test]
fn an_empty_tail_from_the_start_empties_nothing() {
    let all = entries(1, 3);
    let mut book = book(&all);
    assert_eq!(book.append(&LogTail::default(), &view()), 0);
    assert_eq!(book.append(&tail(2, &[]), &view()), 0);
    assert_eq!(book.append(&tail(9, &[]), &view()), 0);
    assert_eq!(book.entries(), &all[..]);
    assert_eq!(book.gaps(), 0, "saying nothing is not a gap");
}

/// A tail that starts past the book's end is refused whole and counted,
/// and the next tail from the start fills the gap.
#[test]
fn a_tail_past_the_end_is_refused_and_counted() {
    let all = entries(1, 6);
    let mut book = book(&all[..2]);
    assert_eq!(book.append(&tail(4, &all[4..]), &view()), 0);
    assert_eq!(book.entries(), &all[..2], "nothing from past the gap");
    assert_eq!(book.gaps(), 1);
    assert_eq!(book.append(&tail(0, &all), &view()), 4);
    assert_eq!(book.entries(), &all[..]);
}

/// A resend from the start that is shorter than the book, and agrees
/// with it, takes nothing away.
#[test]
fn a_shorter_resend_that_agrees_takes_nothing_away() {
    let all = entries(1, 5);
    let mut book = book(&all);
    assert_eq!(book.append(&tail(0, &all[..2]), &view()), 0);
    assert_eq!(book.append(&tail(1, &all[1..3]), &view()), 0);
    assert_eq!(book.entries(), &all[..]);
    assert_eq!(book.rewrites(), 0);
}

/// A tail that disagrees with the book is another game's log: it replaces
/// the book from where the two part, once.
#[test]
fn a_tail_that_disagrees_replaces_the_book_from_where_they_part() {
    let old = entries(1, 5);
    let mut new = old[..2].to_vec();
    new.extend(entries(20, 1));
    let mut book = book(&old);
    assert_eq!(book.append(&tail(0, &new), &view()), 1);
    assert_eq!(book.entries(), &new[..]);
    assert_eq!(book.rewrites(), 1);
}

/// The panel's cursor: new lines are unread until taken, and neither a
/// repeated chunk nor a refused tail moves it.
#[test]
fn lines_are_unread_until_taken() {
    let all = entries(1, 6);
    let roster = roster();
    let wording = Wording {
        lang: Lang::En,
        seat: ME,
        statics: Some(&roster),
        texts: &no_text,
    };
    let mut book = book(&all[..3]);
    assert_eq!(book.unread(), 3);
    book.append(&tail(0, &all[..3]), &view());
    assert_eq!(book.unread(), 3, "a repeated chunk is not news");
    book.append(&tail(5, &all[5..]), &view());
    assert_eq!(book.unread(), 3, "a refused tail reads nothing");
    let taken = book.take_unread(&wording);
    assert_eq!(taken.iter().map(|l| l.index).collect::<Vec<_>>(), [0, 1, 2]);
    assert_eq!(book.unread(), 0);
    assert!(book.take_unread(&wording).is_empty());

    book.append(&tail(3, &all[3..]), &view());
    assert_eq!(book.unread(), 3);
    assert_eq!(book.take_unread(&wording)[0].index, 3);

    // The game-over screen reads the whole log whatever the panel took.
    assert_eq!(book.lines(&wording).len(), 6);
    assert_eq!(book.lines_since(4, &wording).len(), 2);
    book.append(&tail(6, &entries(7, 1)), &view());
    book.mark_read();
    assert_eq!(book.unread(), 0);
}

/// Lines replaced by another game's log are unread again.
#[test]
fn a_rewrite_below_the_cursor_makes_the_new_lines_unread() {
    let mut book = book(&entries(1, 4));
    book.mark_read();
    book.append(&tail(0, &entries(30, 2)), &view());
    assert_eq!(book.unread(), 2);
}

// ---- names -------------------------------------------------------------

/// A card's name is its catalog name in the reader's language, from text
/// that may arrive after the line did, and in whichever language the
/// reader picks.
#[test]
fn a_card_is_named_from_card_text_that_arrives_later() {
    let book = book(&[entry(
        1,
        LogEvent::Cast {
            from: None,
            player: BO,
            spell: bolt(3),
        },
    )]);
    let roster = roster();
    let before = Wording {
        lang: Lang::De,
        seat: ME,
        statics: Some(&roster),
        texts: &no_text,
    };
    assert_eq!(book.lines(&before)[0].text, "Bo hat Lightning Bolt gewirkt");
    assert_eq!(read(&book, Lang::De).text, "Bo hat Blitzschlag gewirkt");
    assert_eq!(read(&book, Lang::En).text, "Bo cast Lightning Bolt");
}

/// A copy is named by the name it has, not by its card's: the catalog
/// text of the card under it would name the wrong card.
#[test]
fn a_copy_keeps_the_name_it_shows() {
    let LogObject::Known { id, card, .. } = bolt(3) else {
        unreachable!()
    };
    let copy = LogObject::Known {
        token: None,
        id,
        card,
        name: "Shock".to_string(),
    };
    assert_eq!(
        say(
            Lang::De,
            LogEvent::Cast {
                from: None,
                player: BO,
                spell: copy
            }
        ),
        "Bo hat Shock gewirkt"
    );
}

/// A card the seat may not see is "a card", a face-down one "a
/// face-down card", and neither line says which object it is.
#[test]
fn a_hidden_or_face_down_card_is_named_neutrally() {
    let face_down = ObjectId::new(77, 3);
    let line = read(
        &book(&[entry(
            1,
            LogEvent::Blocked {
                blocker: LogObject::FaceDown { id: face_down },
                attacker: LogObject::Hidden,
            },
        )]),
        Lang::En,
    );
    assert_eq!(line.text, "A face-down card blocked a card");
    assert!(!line.text.contains("77"), "{}", line.text);
    assert_eq!(
        say(
            Lang::De,
            LogEvent::Discarded {
                player: BO,
                card: LogObject::Hidden
            }
        ),
        "Bo hat eine Karte abgeworfen"
    );
    // The face-down card is on the table for everyone to point at; the
    // hidden one has no handle to point with.
    assert_eq!(line.names.len(), 1);
    assert_eq!(line.names[0].id, face_down);
    assert_eq!(line.names[0].card, None);
}

/// No name is ever empty: an empty catalog name falls back to the name
/// the line carries, and an object with no name at all is "a card".
#[test]
fn a_name_is_never_empty() {
    let blank = |_: CardIndex, _: u8| Some(card_text("", "Lightning Bolt"));
    let roster = roster();
    let wording = Wording {
        lang: Lang::En,
        seat: ME,
        statics: Some(&roster),
        texts: &blank,
    };
    let book = book(&[
        entry(
            1,
            LogEvent::Cast {
                from: None,
                player: BO,
                spell: bolt(3),
            },
        ),
        entry(
            1,
            LogEvent::Created {
                object: token(4, " "),
                controller: BO,
            },
        ),
    ]);
    let lines = book.lines(&wording);
    assert_eq!(lines[0].text, "Bo cast Lightning Bolt");
    assert_eq!(lines[1].text, "Bo created a card");
}

/// Every name span covers exactly the name, wherever the language puts it.
#[test]
fn a_name_span_covers_its_name() {
    let line = read(
        &book(&[entry(
            1,
            LogEvent::Damage {
                source: Some(bolt(3)),
                target: LogTarget::Object(token(4, "Soldier")),
                amount: 3,
                combat: false,
            },
        )]),
        Lang::De,
    );
    assert_eq!(
        line.text,
        "Soldier hat 3 Schaden durch Blitzschlag erlitten"
    );
    let named: Vec<(&str, ObjectId)> = line
        .names
        .iter()
        .map(|span| (&line.text[span.range.clone()], span.id))
        .collect();
    assert_eq!(
        named,
        [
            ("Soldier", ObjectId::new(4, 0)),
            ("Blitzschlag", ObjectId::new(3, 0))
        ]
    );
    assert_eq!(line.names[1].card.map(|c| c.index), Some(BOLT));

    let list = read(
        &book(&[entry(
            1,
            LogEvent::Revealed {
                player: BO,
                cards: vec![token(5, "Clue"), bolt(6), token(7, "Food")],
            },
        )]),
        Lang::En,
    );
    assert_eq!(list.text, "Bo revealed Clue, Lightning Bolt and Food");
    let spans: Vec<&str> = list
        .names
        .iter()
        .map(|span| &list.text[span.range.clone()])
        .collect();
    assert_eq!(spans, ["Clue", "Lightning Bolt", "Food"]);
}

/// A name that reads like a placeholder is written as it is, in the line
/// and in its plain form.
#[test]
fn a_name_that_looks_like_a_placeholder_stays_a_name() {
    let mut roster = roster();
    roster.seats[1].display_name = "{1}".to_string();
    let catalog = Catalog(Lang::En);
    let wording = Wording {
        lang: Lang::En,
        seat: ME,
        statics: Some(&roster),
        texts: &catalog,
    };
    let book = book(&[
        LogEntry {
            at: 0,
            turn: 1,
            repeat: 3,
            event: LogEvent::Cast {
                from: None,
                player: BO,
                spell: bolt(3),
            },
            phase: None,
        },
        entry(
            1,
            LogEvent::Blocked {
                blocker: token(4, "{2}"),
                attacker: bolt(3),
            },
        ),
    ]);
    let lines = book.lines(&wording);
    assert_eq!(lines[0].plain(Lang::En), "{1} cast Lightning Bolt (×3)");
    assert_eq!(lines[1].text, "{2} blocked Lightning Bolt");
}

/// A planeswalker an attack names is called what the view showed when
/// the attack arrived, also after it has left the battlefield, and "a
/// planeswalker" when the view never showed it.
#[test]
fn an_attacked_planeswalker_is_named_as_the_view_showed_it() {
    let walker = printed(40, 1, "Jace Beleren", 3);
    let attack = entry(
        2,
        LogEvent::Attacked {
            attacker: bolt(3),
            defending: Defender::Planeswalker(walker.id),
        },
    );
    let shown = ViewBuilder::new(2)
        .with_battlefield(1, [walker.clone()])
        .build();
    let mut book = LogBook::new();
    book.append(&tail(0, std::slice::from_ref(&attack)), &shown);
    book.append(&tail(1, &entries(3, 1)), &view());
    let line = read(&book, Lang::En);
    assert_eq!(line.text, "Lightning Bolt attacked Jace Beleren");
    assert_eq!(line.names[1].id, walker.id);
    assert_eq!(line.names[1].card, walker.card);

    let fresh = book_with_view(&[attack], &view());
    assert_eq!(
        read(&fresh, Lang::De).text,
        "Blitzschlag hat einen Planeswalker angegriffen"
    );
}

fn book_with_view(entries: &[LogEntry], view: &PlayerView) -> LogBook {
    let mut book = LogBook::new();
    book.append(&tail(0, entries), view);
    book
}

// ---- sentences ---------------------------------------------------------

/// The reading seat is "you", in the verb's form for it; any other seat
/// is named.
#[test]
fn the_reading_seat_is_you_and_the_others_are_named() {
    let cast = |player| LogEvent::Cast {
        from: None,
        player,
        spell: bolt(3),
    };
    assert_eq!(say(Lang::En, cast(ME)), "You cast Lightning Bolt");
    assert_eq!(say(Lang::De, cast(ME)), "Du hast Blitzschlag gewirkt");
    assert_eq!(say(Lang::En, cast(BO)), "Bo cast Lightning Bolt");
    let moved = |owner| LogEvent::Moved {
        place: None,
        object: bolt(3),
        owner,
        from: LogZone::Hand,
        to: LogZone::Graveyard,
    };
    assert_eq!(
        say(Lang::En, moved(ME)),
        "Lightning Bolt moved from your hand into your graveyard"
    );
    assert_eq!(
        say(Lang::De, moved(BO)),
        "Blitzschlag ist aus der Hand von Bo auf den Friedhof von Bo gelangt"
    );
    let hit = |player, combat| LogEvent::Damage {
        source: Some(bolt(3)),
        target: LogTarget::Player(player),
        amount: 3,
        combat,
    };
    assert_eq!(
        say(Lang::En, hit(ME, false)),
        "Lightning Bolt dealt 3 damage to you"
    );
    assert_eq!(
        say(Lang::De, hit(ME, true)),
        "Blitzschlag hat dir 3 Kampfschaden zugefügt"
    );
    assert_eq!(
        say(Lang::En, hit(BO, true)),
        "Lightning Bolt dealt 3 combat damage to Bo"
    );
    let attack = |player| LogEvent::Attacked {
        attacker: token(4, "Soldier"),
        defending: Defender::Player(player),
    };
    assert_eq!(say(Lang::De, attack(ME)), "Soldier hat dich angegriffen");
    assert_eq!(say(Lang::En, attack(BO)), "Soldier attacked Bo");
    assert_eq!(
        say(
            Lang::En,
            LogEvent::ControlChanged {
                object: token(4, "Soldier"),
                old: BO,
                new: ME
            }
        ),
        "You gained control of Soldier"
    );
}

/// A count picks the singular or the plural sentence.
#[test]
fn a_count_picks_its_sentence() {
    let drew = |player, n| LogEvent::Drew {
        player,
        cards: vec![LogObject::Hidden; n],
    };
    assert_eq!(say(Lang::En, drew(ME, 2)), "You drew 2 cards");
    assert_eq!(say(Lang::De, drew(BO, 1)), "Bo hat 1 Karte gezogen");
    let kept = |player, cards| LogEvent::Kept { player, cards };
    assert_eq!(say(Lang::En, kept(ME, 1)), "You kept 1 card");
    assert_eq!(say(Lang::De, kept(BO, 7)), "Bo hat 7 Karten behalten");
    let life = |player, old, new| LogEvent::Life { player, old, new };
    assert_eq!(
        say(Lang::En, life(BO, 20, 17)),
        "Bo went to 17 life (from 20)"
    );
    assert_eq!(
        say(Lang::De, life(ME, 2, 1)),
        "Du hattest danach 1 Lebenspunkt (vorher 2)"
    );
    assert_eq!(
        say(Lang::De, life(ME, 1, -2)),
        "Du hattest danach -2 Lebenspunkte (vorher 1)"
    );
    let counters = |kind, old, new| LogEvent::Counters {
        object: token(4, "Soldier"),
        kind,
        old,
        new,
    };
    assert_eq!(
        say(Lang::En, counters(CounterKind::PLUS_ONE, 1, 2)),
        "Soldier went to 2 +1/+1 counters (from 1)"
    );
    assert_eq!(
        say(Lang::De, counters(CounterKind::Loyalty, 3, 1)),
        "Soldier hatte danach 1 Loyalitätsmarke (vorher 3)"
    );
}

/// A draw the seat may partly see names what it may and counts nothing.
#[test]
fn a_draw_names_what_the_seat_may_see() {
    assert_eq!(
        say(
            Lang::En,
            LogEvent::Drew {
                player: ME,
                cards: vec![bolt(3), LogObject::Hidden]
            }
        ),
        "You drew Lightning Bolt and a card"
    );
}

/// Who won, with the reading seat last among a team, and a draw.
#[test]
fn the_end_of_the_game_names_its_winners() {
    let over = |seats: &[PlayerId]| {
        let mut winners = SeatSet::new();
        for seat in seats {
            winners.insert(*seat);
        }
        LogEvent::GameOver { winners }
    };
    assert_eq!(say(Lang::En, over(&[ME])), "You won the game");
    assert_eq!(say(Lang::De, over(&[BO])), "Bo hat das Spiel gewonnen");
    assert_eq!(say(Lang::En, over(&[ME, BO])), "Bo and you won the game");
    assert_eq!(
        say(Lang::De, over(&[ME, BO])),
        "Bo und du haben das Spiel gewonnen"
    );
    assert_eq!(
        say(Lang::En, over(&[BO, PlayerId::new(2)])),
        "Bo and Cy won the game"
    );
    assert_eq!(say(Lang::De, over(&[])), "Das Spiel endete unentschieden");
}

/// A loss is told in the end screen's words.
#[test]
fn a_loss_is_told_in_the_end_screens_words() {
    let lost = |player| LogEvent::Lost {
        player,
        cause: LossCause::Conceded,
    };
    assert_eq!(
        say(Lang::En, lost(BO)),
        Phrase::LostConcededOther.fill(Lang::En, &["Bo"])
    );
    assert_eq!(
        say(Lang::De, lost(ME)),
        Phrase::LostConcededYou.text(Lang::De)
    );
}

/// A turn opens with a heading, and only a turn does.
#[test]
fn a_turn_opens_with_a_heading() {
    let book = book(&[
        entry(3, LogEvent::TurnStarted { active: BO }),
        entry(3, LogEvent::Shuffled { player: BO }),
        entry(4, LogEvent::TurnStarted { active: ME }),
    ]);
    let roster = roster();
    let wording = Wording {
        lang: Lang::De,
        seat: ME,
        statics: Some(&roster),
        texts: &no_text,
    };
    let lines = book.lines(&wording);
    let seen: Vec<(bool, &str)> = lines.iter().map(|l| (l.header, l.text.as_str())).collect();
    assert_eq!(
        seen,
        [
            (true, "Zug 3 · Bo"),
            (false, "Bo hat die eigene Bibliothek gemischt"),
            (true, "Zug 4 · dein Zug"),
        ]
    );
}

/// A folded line says how often it happened, except a life total or
/// counters, whose "was" already spans every change folded into it.
#[test]
fn a_folded_line_says_how_often_unless_it_spans() {
    let folded = |event| LogEntry {
        at: 0,
        turn: 1,
        repeat: 5,
        event,
        phase: None,
    };
    let book = book(&[
        folded(LogEvent::Cast {
            from: None,
            player: BO,
            spell: bolt(3),
        }),
        folded(LogEvent::Life {
            player: BO,
            old: 20,
            new: 15,
        }),
        folded(LogEvent::Counters {
            object: token(4, "Soldier"),
            kind: CounterKind::PLUS_ONE,
            old: 0,
            new: 5,
        }),
    ]);
    let roster = roster();
    let wording = Wording {
        lang: Lang::En,
        seat: ME,
        statics: Some(&roster),
        texts: &no_text,
    };
    let lines = book.lines(&wording);
    assert_eq!(lines.iter().map(|l| l.times).collect::<Vec<_>>(), [5, 1, 1]);
    assert_eq!(lines[0].plain(Lang::En), "Bo cast Lightning Bolt (×5)");
    assert_eq!(lines[1].plain(Lang::En), "Bo went to 15 life (from 20)");
}

/// An ability line carries the ability, for a panel to show its text.
#[test]
fn an_ability_line_carries_its_ability() {
    let ability = LogAbility {
        ability: None,
        text: None,
        rules: None,
    };
    let line = read(
        &book(&[entry(
            1,
            LogEvent::Ability {
                controller: BO,
                source: token(4, "Soldier"),
                ability: Some(ability),
                activated: None,
            },
        )]),
        Lang::En,
    );
    assert_eq!(line.text, "Bo put an ability of Soldier on the stack");
    assert_eq!(line.ability, Some(ability));
}

/// A line that opens with "a card" opens with a capital, and a seat's
/// name keeps the case its player gave it.
#[test]
fn a_line_capitalizes_our_words_and_never_a_name() {
    let unresolved = LogEvent::DidNotResolve {
        object: LogObject::Hidden,
    };
    assert_eq!(say(Lang::En, unresolved.clone()), "A card did not resolve");
    assert_eq!(
        say(Lang::De, unresolved),
        "Eine Karte wurde nicht verrechnet"
    );
    let mut roster = roster();
    roster.seats[1].display_name = "bo".to_string();
    let wording = Wording {
        lang: Lang::En,
        seat: ME,
        statics: Some(&roster),
        texts: &no_text,
    };
    let book = book(&[entry(1, LogEvent::Shuffled { player: BO })]);
    assert_eq!(book.lines(&wording)[0].text, "bo shuffled their library");
}

/// Which variant an event is, so the table below can show it has them
/// all. No `_` arm: a new event is a compile error here.
fn variant(event: &LogEvent) -> usize {
    match event {
        LogEvent::TurnStarted { .. } => 0,
        LogEvent::Mulliganed { .. } => 1,
        LogEvent::Kept { .. } => 2,
        LogEvent::TimedOut { .. } => 3,
        LogEvent::StandIn { .. } => 4,
        LogEvent::Returned { .. } => 5,
        LogEvent::LandPlayed { .. } => 6,
        LogEvent::Cast { .. } => 7,
        LogEvent::Ability { .. } => 8,
        LogEvent::Countered { .. } => 9,
        LogEvent::DidNotResolve { .. } => 10,
        LogEvent::Drew { .. } => 11,
        LogEvent::Discarded { .. } => 12,
        LogEvent::Moved { .. } => 13,
        LogEvent::Created { .. } => 14,
        LogEvent::Damage { .. } => 15,
        LogEvent::Life { .. } => 16,
        LogEvent::PlayerCounters { .. } => 35,
        LogEvent::Counters { .. } => 17,
        LogEvent::Attacked { .. } => 18,
        LogEvent::Blocked { .. } => 19,
        LogEvent::ControlChanged { .. } => 20,
        LogEvent::TurnedFaceUp { .. } => 29,
        LogEvent::Transformed { .. } => 21,
        LogEvent::Revealed { .. } => 22,
        LogEvent::Shuffled { .. } => 23,
        LogEvent::DiceRolled { .. } => 24,
        LogEvent::Lost { .. } => 25,
        LogEvent::GameOver { .. } => 26,
        LogEvent::LoopDetected { .. } => 27,
        LogEvent::DayNight { .. } => 28,
        LogEvent::Banded { .. } => 30,
        LogEvent::CoinFlipped { .. } => 34,
        LogEvent::CardsKept { .. } => 31,
        LogEvent::BecameMonarch { .. } => 32,
        LogEvent::ConnectionLost { .. } => 33,
        LogEvent::Aimed { .. } => 36,
    }
}

const VARIANTS: usize = 37;

/// Every kind of line, about `player`, with every answer, cause, zone and
/// counter a line can carry.
#[allow(clippy::too_many_lines)]
fn every_line_about(player: PlayerId) -> Vec<LogEvent> {
    let card = bolt(3);
    let mut out = vec![
        LogEvent::PlayerCounters {
            player,
            kind: CounterKind::Energy,
            old: 1,
            new: 2,
        },
        LogEvent::TurnStarted { active: player },
        LogEvent::Mulliganed { player },
        LogEvent::Kept { player, cards: 1 },
        LogEvent::Kept { player, cards: 6 },
        LogEvent::StandIn { player },
        LogEvent::ConnectionLost {
            player,
            wait_secs: Some(161),
        },
        LogEvent::ConnectionLost {
            player,
            wait_secs: None,
        },
        LogEvent::Returned { player },
        LogEvent::LandPlayed {
            from: None,
            player,
            land: token(5, "Island"),
        },
        LogEvent::Cast {
            from: None,
            player,
            spell: card.clone(),
        },
        LogEvent::Ability {
            controller: player,
            source: LogObject::FaceDown {
                id: ObjectId::new(9, 1),
            },
            ability: None,
            activated: None,
        },
        LogEvent::Aimed {
            controller: player,
            source: card.clone(),
            ability: false,
            rules: Some(RulesFace {
                card: BOLT,
                face: 0,
            }),
            modes: 0b10,
            targets: vec![
                LogTarget::Player(player),
                LogTarget::Object(token(5, "Island")),
            ],
        },
        LogEvent::Countered {
            spell: card.clone(),
        },
        LogEvent::DidNotResolve {
            object: LogObject::Hidden,
        },
        LogEvent::Drew {
            player,
            cards: vec![LogObject::Hidden],
        },
        LogEvent::Drew {
            player,
            cards: vec![LogObject::Hidden; 3],
        },
        LogEvent::Drew {
            player,
            cards: vec![card.clone(), token(6, "Clue")],
        },
        LogEvent::Discarded {
            player,
            card: card.clone(),
        },
        LogEvent::Created {
            object: token(6, "Clue"),
            controller: player,
        },
        LogEvent::Life {
            player,
            old: 2,
            new: 1,
        },
        LogEvent::Life {
            player,
            old: 20,
            new: 17,
        },
        LogEvent::Attacked {
            attacker: token(4, "Soldier"),
            defending: Defender::Player(player),
        },
        LogEvent::Attacked {
            attacker: token(4, "Soldier"),
            defending: Defender::Planeswalker(ObjectId::new(40, 0)),
        },
        LogEvent::Blocked {
            blocker: token(4, "Soldier"),
            attacker: card.clone(),
        },
        LogEvent::Banded {
            attacker: token(4, "Soldier"),
            with: token(7, "Knight"),
        },
        LogEvent::ControlChanged {
            object: card.clone(),
            old: PlayerId::new(2),
            new: player,
        },
        LogEvent::TurnedFaceUp {
            object: card.clone(),
        },
        LogEvent::Transformed {
            object: card.clone(),
        },
        LogEvent::CardsKept {
            player,
            cards: vec![card.clone()],
        },
        LogEvent::Revealed {
            player,
            cards: vec![card.clone()],
        },
        LogEvent::Shuffled { player },
        LogEvent::BecameMonarch { player },
        LogEvent::DiceRolled {
            player,
            sides: 20,
            result: 17,
        },
        LogEvent::CoinFlipped { player, won: true },
        LogEvent::CoinFlipped { player, won: false },
        LogEvent::GameOver {
            winners: SeatSet::new(),
        },
        LogEvent::LoopDetected { broken: true },
        LogEvent::LoopDetected { broken: false },
        LogEvent::DayNight { now: DayNight::Day },
        LogEvent::DayNight {
            now: DayNight::Night,
        },
    ];
    for answer in [
        ClockAnswer::Passed,
        ClockAnswer::Kept,
        ClockAnswer::NoAttackers,
        ClockAnswer::NoBlockers,
        ClockAnswer::Declined,
        ClockAnswer::ChosenForThem,
    ] {
        out.push(LogEvent::TimedOut { player, answer });
    }
    for cause in [
        LossCause::Life,
        LossCause::EmptyDraw,
        LossCause::Poison,
        LossCause::CommanderDamage,
        LossCause::Conceded,
        LossCause::Effect,
    ] {
        out.push(LogEvent::Lost { player, cause });
    }
    let zones = [
        LogZone::Library,
        LogZone::Hand,
        LogZone::Battlefield,
        LogZone::Graveyard,
        LogZone::Exile,
        LogZone::Command,
        LogZone::Stack,
    ];
    for from in zones {
        for to in zones {
            out.push(LogEvent::Moved {
                place: None,
                object: card.clone(),
                owner: player,
                from,
                to,
            });
        }
    }
    for place in [
        LogPlace::Top,
        LogPlace::Bottom,
        LogPlace::FromTop(2),
        LogPlace::FromTop(11),
        LogPlace::FromTop(23),
        LogPlace::Shuffled,
    ] {
        out.push(LogEvent::Moved {
            place: Some(place),
            object: card.clone(),
            owner: player,
            from: LogZone::Hand,
            to: LogZone::Library,
        });
    }
    for (zone, owner) in [
        (LogZone::Hand, player),
        (LogZone::Library, player),
        (LogZone::Graveyard, player),
        (LogZone::Graveyard, PlayerId::new(2)),
        (LogZone::Exile, PlayerId::new(2)),
        (LogZone::Command, player),
    ] {
        let from = Some(LogFrom { zone, owner });
        out.push(LogEvent::LandPlayed {
            player,
            land: token(5, "Island"),
            from,
        });
        out.push(LogEvent::Cast {
            player,
            spell: card.clone(),
            from,
        });
    }
    for (source, combat) in [
        (Some(card.clone()), true),
        (Some(card), false),
        (None, false),
    ] {
        for target in [
            LogTarget::Player(player),
            LogTarget::Object(token(4, "Soldier")),
        ] {
            out.push(LogEvent::Damage {
                source: source.clone(),
                target,
                amount: 2,
                combat,
            });
        }
    }
    for kind in [
        CounterKind::PLUS_ONE,
        CounterKind::Minus {
            power: 2,
            toughness: 1,
        },
        CounterKind::Loyalty,
        CounterKind::Lore,
        CounterKind::Time,
        CounterKind::Charge,
        CounterKind::Poison,
        CounterKind::Energy,
        CounterKind::Rad,
        CounterKind::Lifelink,
        CounterKind::Level,
        CounterKind::Custom(99),
    ] {
        for new in [1, 4] {
            out.push(LogEvent::Counters {
                object: token(4, "Soldier"),
                kind,
                old: 2,
                new,
            });
        }
    }
    for winners in [&[player][..], &[player, PlayerId::new(2)]] {
        let mut set = SeatSet::new();
        for seat in winners {
            set.insert(*seat);
        }
        out.push(LogEvent::GameOver { winners: set });
    }
    out
}

/// Every kind of line, about the reading seat and about another, in every
/// language, is a whole sentence: no placeholder left, no gap where a
/// name went, no full stop, and the reading seat never by its name.
#[test]
fn every_line_is_a_whole_sentence_in_every_language() {
    let mut seen = [false; VARIANTS];
    for player in [ME, BO] {
        let events = every_line_about(player);
        for event in &events {
            seen[variant(event)] = true;
        }
        let entries: Vec<LogEntry> = events.into_iter().map(|e| entry(1, e)).collect();
        let book = book(&entries);
        for lang in Lang::ALL {
            let roster = roster();
            let catalog = Catalog(lang);
            let wording = Wording {
                lang,
                seat: ME,
                statics: Some(&roster),
                texts: &catalog,
            };
            for (line, entry) in book.lines(&wording).iter().zip(&entries) {
                let text = &line.text;
                let about = format!("{:?} in {lang:?}: {text:?}", entry.event);
                assert!(!text.trim().is_empty(), "{about}");
                assert!(!text.contains(['{', '}']), "{about}");
                assert!(!text.contains("  "), "{about}");
                assert_eq!(text.trim(), text, "{about}");
                assert!(!text.ends_with('.'), "{about}");
                assert!(!text.starts_with(char::is_lowercase), "{about}");
                assert!(!text.contains("Ada"), "the reader by name: {about}");
                for span in &line.names {
                    assert!(text.get(span.range.clone()).is_some(), "{about}");
                }
            }
        }
    }
    let missing: Vec<usize> = (0..VARIANTS).filter(|&i| !seen[i]).collect();
    assert!(missing.is_empty(), "no line of variants {missing:?}");
}

/// The seat each kind of line is about: the player it names, or whose
/// card moved; nobody for a line about the table. No `_` arm: a new event
/// has to say.
fn about_whom(event: &LogEvent) -> Option<PlayerId> {
    match event {
        LogEvent::TurnStarted { active: player }
        | LogEvent::Mulliganed { player }
        | LogEvent::Kept { player, .. }
        | LogEvent::TimedOut { player, .. }
        | LogEvent::StandIn { player }
        | LogEvent::ConnectionLost { player, .. }
        | LogEvent::Returned { player }
        | LogEvent::LandPlayed { player, .. }
        | LogEvent::Cast { player, .. }
        | LogEvent::Ability {
            controller: player, ..
        }
        | LogEvent::Aimed {
            controller: player, ..
        }
        | LogEvent::Drew { player, .. }
        | LogEvent::Discarded { player, .. }
        | LogEvent::Moved { owner: player, .. }
        | LogEvent::Created {
            controller: player, ..
        }
        | LogEvent::Damage {
            target: LogTarget::Player(player),
            ..
        }
        | LogEvent::Life { player, .. }
        | LogEvent::PlayerCounters { player, .. }
        | LogEvent::Attacked {
            defending: Defender::Player(player),
            ..
        }
        | LogEvent::ControlChanged { new: player, .. }
        | LogEvent::CardsKept { player, .. }
        | LogEvent::Revealed { player, .. }
        | LogEvent::Shuffled { player }
        | LogEvent::BecameMonarch { player }
        | LogEvent::DiceRolled { player, .. }
        | LogEvent::CoinFlipped { player, .. }
        | LogEvent::Lost { player, .. } => Some(*player),
        LogEvent::Countered { .. }
        | LogEvent::DidNotResolve { .. }
        | LogEvent::Damage {
            target: LogTarget::Object(_),
            ..
        }
        | LogEvent::Counters { .. }
        | LogEvent::Attacked {
            defending: Defender::Planeswalker(_) | Defender::Battle(_),
            ..
        }
        | LogEvent::Blocked { .. }
        | LogEvent::Banded { .. }
        | LogEvent::TurnedFaceUp { .. }
        | LogEvent::Transformed { .. }
        | LogEvent::GameOver { .. }
        | LogEvent::LoopDetected { .. }
        | LogEvent::DayNight { .. } => None,
    }
}

/// Every line says which seat it is about, the reading seat as much as
/// any other, so a panel can mark it; a line about the table says
/// nobody, even the end of a game one seat won.
#[test]
fn every_line_says_which_seat_it_is_about() {
    let cast = LogEvent::Cast {
        from: None,
        player: BO,
        spell: bolt(3),
    };
    assert_eq!(read(&book(&[entry(1, cast)]), Lang::En).subject, Some(BO));
    let day = LogEvent::DayNight { now: DayNight::Day };
    assert_eq!(read(&book(&[entry(1, day)]), Lang::En).subject, None);

    let roster = roster();
    let wording = Wording {
        lang: Lang::En,
        seat: ME,
        statics: Some(&roster),
        texts: &no_text,
    };
    for player in [ME, BO] {
        let entries: Vec<LogEntry> = every_line_about(player)
            .into_iter()
            .map(|event| entry(1, event))
            .collect();
        let lines = book(&entries).lines(&wording);
        for (line, entry) in lines.iter().zip(&entries) {
            assert_eq!(
                line.subject,
                about_whom(&entry.event),
                "{:?}: {:?}",
                entry.event,
                line.text
            );
        }
        let about = lines.iter().filter(|line| line.subject == Some(player));
        assert!(about.count() > 40, "the walk names {player:?}");
    }
}

/// Every player a line names is marked where it names them (#300):
/// another seat by its name wherever the sentence puts it, and the
/// reading seat wherever the sentence says "you" to it, and nowhere
/// else: not "your", and never inside a card's name.
#[test]
fn every_player_a_line_names_is_marked() {
    let roster = roster();
    let you: fn(Lang) -> &'static [&'static str] = |lang| match lang {
        Lang::En => &["you", "You"],
        Lang::De => &["du", "Du", "dich", "dir"],
    };
    let mut marked_you = 0;
    for lang in Lang::ALL {
        let wording = Wording {
            lang,
            seat: ME,
            statics: Some(&roster),
            texts: &no_text,
        };
        for player in [ME, BO] {
            let entries: Vec<LogEntry> = every_line_about(player)
                .into_iter()
                .map(|event| entry(1, event))
                .collect();
            for line in book(&entries).lines(&wording) {
                let in_a_name = |at: usize| line.names.iter().any(|n| n.range.contains(&at));
                let mut words = Vec::new();
                let mut start = None;
                for (at, c) in line.text.char_indices().chain([(line.text.len(), ' ')]) {
                    match (c.is_alphanumeric(), start) {
                        (true, None) => start = Some(at),
                        (false, Some(from)) => {
                            words.push(from..at);
                            start = None;
                        }
                        _ => {}
                    }
                }
                let naming: Vec<Range<usize>> = words
                    .into_iter()
                    .filter(|word| !in_a_name(word.start))
                    .filter(|word| {
                        let text = &line.text[word.clone()];
                        you(lang).contains(&text) || text == "Bo" || text == "Cy"
                    })
                    .collect();
                let marks: Vec<Range<usize>> =
                    line.players.iter().map(|span| span.range.clone()).collect();
                assert_eq!(marks, naming, "{lang:?}: {}", line.text);
                for span in &line.players {
                    let text = &line.text[span.range.clone()];
                    if span.player == ME {
                        assert!(you(lang).contains(&text), "{}", line.text);
                        marked_you += 1;
                    } else {
                        assert_eq!(text, seat_name(lang, Some(&roster), span.player));
                    }
                }
            }
        }
    }
    assert!(marked_you > 80, "the walk says you {marked_you} times");
}

/// A card put into a library says where in it (#300), and a card
/// shuffled in says that instead.
#[test]
fn a_card_put_into_a_library_says_where() {
    let moved = |owner, place| LogEvent::Moved {
        object: bolt(3),
        owner,
        from: LogZone::Hand,
        to: LogZone::Library,
        place: Some(place),
    };
    assert_eq!(
        say(Lang::En, moved(ME, LogPlace::Top)),
        "Lightning Bolt moved from your hand on top of your library"
    );
    assert_eq!(
        say(Lang::En, moved(BO, LogPlace::Bottom)),
        "Lightning Bolt moved from Bo's hand on the bottom of Bo's library"
    );
    for (n, nth) in [
        (2, "2nd"),
        (3, "3rd"),
        (4, "4th"),
        (11, "11th"),
        (12, "12th"),
        (21, "21st"),
        (113, "113th"),
    ] {
        assert_eq!(
            say(Lang::En, moved(ME, LogPlace::FromTop(n))),
            format!("Lightning Bolt moved from your hand {nth} from the top of your library")
        );
    }
    assert_eq!(
        say(Lang::De, moved(BO, LogPlace::FromTop(3))),
        "Blitzschlag ist aus der Hand von Bo als 3. Karte von oben in die Bibliothek von Bo gelangt"
    );
    assert_eq!(
        say(Lang::En, moved(ME, LogPlace::Shuffled)),
        "Lightning Bolt was shuffled from your hand into your library"
    );
    assert_eq!(
        say(Lang::De, moved(ME, LogPlace::Shuffled)),
        "Blitzschlag wurde aus deiner Hand in deine Bibliothek gemischt"
    );
}

/// A land played and a spell cast say where from (#300): "their" when the
/// zone is the player's own, the owner's name when it is not, and
/// nothing when the host could not say.
#[test]
fn a_play_and_a_cast_say_where_from() {
    let cast = |player, from| LogEvent::Cast {
        player,
        spell: bolt(3),
        from,
    };
    let from = |zone, owner| Some(LogFrom { zone, owner });
    assert_eq!(
        say(
            Lang::En,
            LogEvent::LandPlayed {
                player: ME,
                land: token(5, "Island"),
                from: from(LogZone::Hand, ME),
            }
        ),
        "You played Island from your hand"
    );
    assert_eq!(
        say(Lang::En, cast(BO, from(LogZone::Graveyard, BO))),
        "Bo cast Lightning Bolt from their graveyard"
    );
    assert_eq!(
        say(Lang::De, cast(BO, from(LogZone::Graveyard, BO))),
        "Bo hat Blitzschlag aus dem eigenen Friedhof gewirkt"
    );
    assert_eq!(
        say(Lang::En, cast(BO, from(LogZone::Graveyard, ME))),
        "Bo cast Lightning Bolt from your graveyard"
    );
    assert_eq!(
        say(Lang::En, cast(ME, from(LogZone::Graveyard, BO))),
        "You cast Lightning Bolt from Bo's graveyard"
    );
    assert_eq!(
        say(Lang::De, cast(ME, from(LogZone::Exile, BO))),
        "Du hast Blitzschlag aus dem Exil gewirkt"
    );
    assert_eq!(say(Lang::En, cast(BO, None)), "Bo cast Lightning Bolt");
}

/// A line's time is said as the wall clock says it, wherever the device
/// is: the offset moves it across midnight either way, and a half-hour
/// zone keeps its minutes (#300).
#[test]
fn a_line_says_its_time_on_the_wall_clock() {
    let mut line = LogLine {
        index: 0,
        turn: 1,
        times: 1,
        header: false,
        at: 0,
        text: String::new(),
        names: Vec::new(),
        players: Vec::new(),
        ability: None,
        subject: None,
        crowned: false,
        phase: None,
        follows: false,
    };
    assert_eq!(line.clock(0), None, "a line the host never dated");
    // 2026-09-25 12:34:56.789 UTC.
    line.at = 1_790_339_696_789;
    assert_eq!(line.clock(0).as_deref(), Some("12:34"));
    assert_eq!(line.clock(2 * 3600).as_deref(), Some("14:34"));
    // West of Greenwich across midnight, and east across it.
    assert_eq!(line.clock(-13 * 3600).as_deref(), Some("23:34"));
    assert_eq!(line.clock(12 * 3600).as_deref(), Some("00:34"));
    assert_eq!(line.clock(5 * 3600 + 1800).as_deref(), Some("18:04"));
}

/// A line carries the time the host wrote it, and a token's registry
/// entry, so a panel can show both after the token is gone (#300).
#[test]
fn a_line_carries_its_time_and_its_token() {
    let treasure = LogObject::Known {
        id: ObjectId::new(12, 0),
        card: None,
        token: Some(3),
        name: "Treasure".to_string(),
    };
    let entry = LogEntry {
        at: 1_727_000_000_000,
        ..entry(
            2,
            LogEvent::Created {
                object: treasure,
                controller: BO,
            },
        )
    };
    let line = read(&book(&[entry]), Lang::En);
    assert_eq!(line.at, 1_727_000_000_000);
    assert_eq!(line.names.len(), 1);
    assert_eq!(line.names[0].token, Some(3));
    assert_eq!(&line.text[line.names[0].range.clone()], "Treasure");
}

/// Every log phrase has a line that says it: a phrase nothing writes is a
/// sentence a translator keeps for nobody.
#[test]
fn every_log_phrase_is_written_by_this_module() {
    let source = include_str!("../gamelog.rs");
    let (code, _) = source
        .split_once("#[cfg(test)]")
        .expect("the tests follow the code");
    let unused: Vec<String> = Phrase::ALL
        .iter()
        .map(|phrase| format!("{phrase:?}"))
        .filter(|name| name.starts_with("Log"))
        // The panel captions a turn's phases itself: the log only
        // carries the phase, and `baylee-client`'s phase-caption test
        // holds these.
        .filter(|name| !name.starts_with("LogPhase"))
        .filter(|name| !code.contains(&format!("Phrase::{name}")))
        .collect();
    assert!(unused.is_empty(), "never written: {unused:?}");
}

// ---- modes, targets, activations (the log, 12.10.2026) -----------------

/// Card text that can say a mode's words: "Words of mode N" for the
/// modes of Lightning Bolt's card, and nothing for any other card.
struct Modal;

impl CardTextLookup for Modal {
    fn text(&self, card: CardIndex, face: u8) -> Option<CardText> {
        Catalog(Lang::En).text(card, face)
    }

    fn mode(&self, rules: RulesFace, mode: usize) -> Option<String> {
        (rules.card == BOLT).then(|| format!("Words of mode {mode}"))
    }
}

/// Line `index` of `book` as Ada reads it in `lang`, with `texts`.
fn read_at(book: &LogBook, index: usize, lang: Lang, texts: &dyn CardTextLookup) -> LogLine {
    let roster = roster();
    let wording = Wording {
        lang,
        seat: ME,
        statics: Some(&roster),
        texts,
    };
    book.line(index, &wording).expect("the book holds the line")
}

fn cast(player: PlayerId, spell: LogObject) -> LogEvent {
    LogEvent::Cast {
        player,
        spell,
        from: None,
    }
}

fn aimed(
    controller: PlayerId,
    source: LogObject,
    ability: bool,
    modes: u8,
    targets: Vec<LogTarget>,
) -> LogEvent {
    LogEvent::Aimed {
        controller,
        source,
        ability,
        rules: (modes != 0).then_some(RulesFace {
            card: BOLT,
            face: 0,
        }),
        modes,
        targets,
    }
}

fn activation(controller: PlayerId, source: LogObject, activated: Option<bool>) -> LogEvent {
    LogEvent::Ability {
        controller,
        source,
        ability: None,
        activated,
    }
}

/// The aim right under the cast of its spell goes on from it: only the
/// parts, the line above having named the spell, and about whoever cast
/// it. Read alone, it names the spell itself.
#[test]
fn an_aim_under_its_cast_carries_on_from_it() {
    let at_bo = || aimed(BO, bolt(3), false, 0, vec![LogTarget::Player(BO)]);
    let under = book(&[entry(1, cast(BO, bolt(3))), entry(1, at_bo())]);
    let line = read_at(&under, 1, Lang::En, &Catalog(Lang::En));
    assert!(line.follows);
    assert_eq!(line.text, "targeting Bo");
    assert_eq!(line.subject, Some(BO));
    assert_eq!(
        read_at(&under, 1, Lang::De, &Catalog(Lang::De)).text,
        "Ziel: Bo"
    );
    assert!(!read_at(&under, 0, Lang::En, &Catalog(Lang::En)).follows);

    let at_me = book(&[
        entry(1, cast(BO, bolt(3))),
        entry(1, aimed(BO, bolt(3), false, 0, vec![LogTarget::Player(ME)])),
    ]);
    assert_eq!(
        read_at(&at_me, 1, Lang::En, &Catalog(Lang::En)).text,
        "targeting you"
    );

    let alone = book(&[
        entry(1, LogEvent::Shuffled { player: BO }),
        entry(1, at_bo()),
    ]);
    let line = read_at(&alone, 1, Lang::En, &Catalog(Lang::En));
    assert!(!line.follows);
    assert_eq!(line.text, "Lightning Bolt: targeting Bo");
    assert_eq!(
        read_at(&alone, 1, Lang::De, &Catalog(Lang::De)).text,
        "Blitzschlag: Ziel: Bo"
    );
}

/// An aim carries on only from the line of its own spell or ability, in
/// the same turn, unfolded: not from another object's cast, not from the
/// cast of a spell when it is the aim of an ability (or the other way
/// round), and not where the object is not named at all.
#[test]
fn an_aim_carries_on_only_from_its_own_object() {
    let follows = |before: LogEvent, after: LogEvent| {
        read_at(
            &book(&[entry(1, before), entry(1, after)]),
            1,
            Lang::En,
            &no_text,
        )
        .follows
    };
    let at = |source, ability| aimed(BO, source, ability, 0, vec![LogTarget::Player(ME)]);
    assert!(follows(cast(BO, bolt(3)), at(bolt(3), false)));
    assert!(!follows(cast(BO, bolt(4)), at(bolt(3), false)), "another");
    assert!(!follows(cast(BO, bolt(3)), at(bolt(3), true)), "spell");
    assert!(
        follows(activation(BO, bolt(3), Some(false)), at(bolt(3), true)),
        "an ability goes on from its own line"
    );
    assert!(!follows(
        activation(BO, bolt(3), Some(false)),
        at(bolt(3), false)
    ));
    assert!(!follows(
        cast(BO, LogObject::Hidden),
        at(LogObject::Hidden, false)
    ));
    // Another turn, or a line folded from several.
    let later = LogEntry {
        turn: 2,
        ..entry(1, at(bolt(3), false))
    };
    assert!(
        !read_at(
            &book(&[entry(1, cast(BO, bolt(3))), later]),
            1,
            Lang::En,
            &no_text
        )
        .follows
    );
    for (cast_times, aim_times) in [(2, 1), (1, 2)] {
        let book = book(&[
            LogEntry {
                repeat: cast_times,
                ..entry(1, cast(BO, bolt(3)))
            },
            LogEntry {
                repeat: aim_times,
                ..entry(1, at(bolt(3), false))
            },
        ]);
        assert!(!read_at(&book, 1, Lang::En, &no_text).follows);
    }
}

/// An aim that is no continuation names its spell, or says it is an
/// ability's.
#[test]
fn an_aim_on_its_own_names_what_it_is_the_aim_of() {
    let line = |ability| {
        let book = book(&[
            entry(1, LogEvent::Shuffled { player: BO }),
            entry(
                1,
                aimed(BO, bolt(3), ability, 0, vec![LogTarget::Player(BO)]),
            ),
        ]);
        read_at(&book, 1, Lang::En, &Catalog(Lang::En)).text
    };
    assert_eq!(line(false), "Lightning Bolt: targeting Bo");
    assert_eq!(line(true), "Ability of Lightning Bolt: targeting Bo");
}

/// Several targets read as a list, and a target that is an object is
/// named as an object is.
#[test]
fn an_aim_at_several_targets_lists_them() {
    let book = book(&[
        entry(1, cast(BO, bolt(3))),
        entry(
            1,
            aimed(
                BO,
                bolt(3),
                false,
                0,
                vec![
                    LogTarget::Player(ME),
                    LogTarget::Object(token(5, "Island")),
                    LogTarget::Player(PlayerId::new(2)),
                ],
            ),
        ),
    ]);
    let line = read_at(&book, 1, Lang::En, &no_text);
    assert_eq!(line.text, "targeting you, Island and Cy");
    assert_eq!(
        read_at(&book, 1, Lang::De, &no_text).text,
        "Ziele: du, Island und Cy"
    );
}

/// A chosen mode is said in the card's own words, quoted, when the
/// client can say them; several are joined, and modes and targets are
/// two parts of one line.
#[test]
fn a_chosen_mode_is_said_in_the_cards_words() {
    let line = |lang, modes, targets: Vec<LogTarget>| {
        let book = book(&[
            entry(1, cast(BO, bolt(3))),
            entry(1, aimed(BO, bolt(3), false, modes, targets)),
        ]);
        read_at(&book, 1, lang, &Modal).text
    };
    assert_eq!(
        line(Lang::En, 0b10, Vec::new()),
        "mode: \u{201c}Words of mode 1\u{201d}"
    );
    assert_eq!(
        line(Lang::De, 0b10, Vec::new()),
        "Modus: \u{201e}Words of mode 1\u{201c}"
    );
    assert_eq!(
        line(Lang::En, 0b101, Vec::new()),
        "modes: \u{201c}Words of mode 0\u{201d} + \u{201c}Words of mode 2\u{201d}"
    );
    assert_eq!(
        line(Lang::En, 0b1, vec![LogTarget::Player(BO)]),
        "mode: \u{201c}Words of mode 0\u{201d}; targeting Bo"
    );
}

/// Without the words of a mode the line says which it was, counting from
/// one as a player counts the bullets.
#[test]
fn a_mode_without_words_is_said_by_its_number() {
    let book = book(&[
        entry(1, cast(BO, bolt(3))),
        entry(1, aimed(BO, bolt(3), false, 0b10, Vec::new())),
    ]);
    assert_eq!(
        read_at(&book, 1, Lang::En, &Catalog(Lang::En)).text,
        "mode: Mode 2"
    );
    assert_eq!(
        read_at(&book, 1, Lang::De, &Catalog(Lang::De)).text,
        "Modus: Modus 2"
    );
}

/// A player's life total right under the damage that took it says where
/// it stands now, and only then.
#[test]
fn life_under_its_damage_says_where_it_stands() {
    let hit = |target| LogEvent::Damage {
        source: Some(bolt(3)),
        target,
        amount: 3,
        combat: false,
    };
    let life = |player, old, new| LogEvent::Life { player, old, new };
    let second =
        |before: LogEntry, after: LogEntry| read_at(&book(&[before, after]), 1, Lang::En, &no_text);

    let line = second(
        entry(1, hit(LogTarget::Player(BO))),
        entry(1, life(BO, 20, 17)),
    );
    assert!(line.follows);
    assert_eq!(line.text, "now at 17 life (from 20)");
    assert_eq!(line.subject, Some(BO));
    let german = read_at(
        &book(&[
            entry(1, hit(LogTarget::Player(BO))),
            entry(1, life(BO, 20, 17)),
        ]),
        1,
        Lang::De,
        &no_text,
    );
    assert_eq!(german.text, "jetzt 17 Lebenspunkte (vorher 20)");

    let apart = [
        (
            "another player",
            entry(1, hit(LogTarget::Player(ME))),
            entry(1, life(BO, 20, 17)),
        ),
        (
            "a permanent",
            entry(1, hit(LogTarget::Object(token(5, "Island")))),
            entry(1, life(BO, 20, 17)),
        ),
        (
            "gain",
            entry(1, hit(LogTarget::Player(BO))),
            entry(1, life(BO, 17, 20)),
        ),
        (
            "another amount (lifelink, a second source)",
            entry(1, hit(LogTarget::Player(BO))),
            entry(1, life(BO, 20, 19)),
        ),
        (
            "a folded damage line",
            LogEntry {
                repeat: 2,
                ..entry(1, hit(LogTarget::Player(BO)))
            },
            entry(1, life(BO, 20, 17)),
        ),
        (
            "a folded life line",
            entry(1, hit(LogTarget::Player(BO))),
            LogEntry {
                repeat: 2,
                ..entry(1, life(BO, 20, 17))
            },
        ),
    ];
    for (why, before, after) in apart {
        let line = second(before, after);
        assert!(!line.follows, "{why}");
        let (old, new) = match why {
            "gain" => (17, 20),
            "another amount (lifelink, a second source)" => (20, 19),
            _ => (20, 17),
        };
        assert_eq!(
            line.text,
            format!("Bo went to {new} life (from {old})"),
            "{why}"
        );
    }
}

/// A permanent that had no counters of a kind got them; one that had
/// some went to a new total.
#[test]
fn counters_from_nothing_are_got() {
    let counters = |kind, old, new| LogEvent::Counters {
        object: token(4, "Soldier"),
        kind,
        old,
        new,
    };
    assert_eq!(
        say(Lang::En, counters(CounterKind::Loyalty, 0, 4)),
        "Soldier got 4 loyalty counters"
    );
    assert_eq!(
        say(Lang::En, counters(CounterKind::Loyalty, 3, 4)),
        "Soldier went to 4 loyalty counters (from 3)"
    );
    assert_eq!(
        say(Lang::De, counters(CounterKind::Loyalty, 0, 4)),
        "Soldier hat 4 Loyalitätsmarken bekommen"
    );
}

/// An ability says whether it was activated or triggered, for the seat
/// that reads and for the others; one the host could not say keeps the
/// plain wording.
#[test]
fn an_ability_says_whether_it_was_activated_or_triggered() {
    let say_it =
        |lang, controller, activated| say(lang, activation(controller, bolt(3), activated));
    assert_eq!(
        say_it(Lang::En, BO, Some(true)),
        "Bo activated Lightning Bolt"
    );
    assert_eq!(
        say_it(Lang::En, ME, Some(true)),
        "You activated Lightning Bolt"
    );
    assert_eq!(
        say_it(Lang::En, BO, Some(false)),
        "Lightning Bolt triggered for Bo"
    );
    assert_eq!(
        say_it(Lang::En, ME, Some(false)),
        "Lightning Bolt triggered for you"
    );
    assert_eq!(
        say_it(Lang::En, BO, None),
        "Bo put an ability of Lightning Bolt on the stack"
    );
    assert_eq!(
        say_it(Lang::De, BO, Some(true)),
        "Bo hat Blitzschlag aktiviert"
    );
    assert_eq!(
        say_it(Lang::De, ME, Some(true)),
        "Du hast Blitzschlag aktiviert"
    );
    assert_eq!(
        say_it(Lang::De, BO, Some(false)),
        "Blitzschlag hat für Bo ausgelöst"
    );
    assert_eq!(
        say_it(Lang::De, ME, Some(false)),
        "Blitzschlag hat für dich ausgelöst"
    );
}

/// The modes the book last heard for an object this turn, of the spell
/// or of its ability, and none for a turn ago or an unnamed source.
#[test]
fn the_book_remembers_the_modes_of_this_turn() {
    let face = RulesFace {
        card: BOLT,
        face: 0,
    };
    let id = |slot| ObjectId::new(slot, 0);
    let aim = |source, ability, modes| entry(1, aimed(BO, source, ability, modes, Vec::new()));
    let heard = book(&[
        entry(1, LogEvent::TurnStarted { active: ME }),
        entry(1, cast(BO, bolt(3))),
        aim(bolt(3), false, 0b10),
        aim(bolt(3), true, 0b100),
        aim(bolt(4), false, 0b1),
    ]);
    assert_eq!(heard.modes_of(id(3), false), Some((face, 0b10)));
    assert_eq!(heard.modes_of(id(3), true), Some((face, 0b100)));
    assert_eq!(heard.modes_of(id(4), false), Some((face, 0b1)));
    assert_eq!(
        heard.modes_of(id(4), true),
        None,
        "a spell's, not an ability's"
    );
    assert_eq!(heard.modes_of(id(9), false), None, "another object");

    // The latest line wins: a mode changed on the stack.
    let changed = book(&[
        entry(1, LogEvent::TurnStarted { active: ME }),
        aim(bolt(3), false, 0b10),
        aim(bolt(3), false, 0b1),
    ]);
    assert_eq!(changed.modes_of(id(3), false), Some((face, 0b1)));

    // Nothing stays on the stack from one turn into the next.
    let past = book(&[
        entry(1, LogEvent::TurnStarted { active: ME }),
        aim(bolt(3), false, 0b10),
        entry(2, LogEvent::TurnStarted { active: BO }),
    ]);
    assert_eq!(past.modes_of(id(3), false), None);

    // A source the seat may not know, or an aim with no modes.
    let unknown = book(&[
        entry(1, LogEvent::TurnStarted { active: ME }),
        aim(LogObject::Hidden, false, 0b10),
        aim(bolt(3), false, 0),
    ]);
    assert_eq!(unknown.modes_of(id(3), false), None);
}

/// A line says the phase its entry was written in.
#[test]
fn a_line_carries_its_phase() {
    let shuffled = LogEvent::Shuffled { player: BO };
    let phased = LogEntry {
        phase: Some(Phase::Combat),
        ..entry(1, shuffled.clone())
    };
    assert_eq!(read(&book(&[phased]), Lang::En).phase, Some(Phase::Combat));
    assert_eq!(read(&book(&[entry(1, shuffled)]), Lang::En).phase, None);
}
