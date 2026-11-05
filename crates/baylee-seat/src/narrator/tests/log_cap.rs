//! The cap on the log lines a wake tells (`LOG_LINES`): it counts the lines
//! as written, after identical ones are folded, not the entries heard.

use super::*;

/// `count` discards by the opponent, the `n`th of a card named `name(n)`.
fn discards(count: u32, name: impl Fn(u32) -> String) -> LogTail {
    LogTail {
        from: 0,
        entries: (0..count)
            .map(|n| {
                line(
                    7,
                    LogEvent::Discarded {
                        player: THEM,
                        card: LogObject::Known {
                            id: id(300 + n),
                            card: None,
                            token: None,
                            name: name(n),
                        },
                    },
                )
            })
            .collect(),
    }
}

fn told(log: &LogTail) -> String {
    let (view, _) = board();
    let pending = priority(&view);
    let request = request(view, pending, log.clone());
    let mut narrator = Narrator::new(&request.context);
    narrator.hear(log);
    narrator.wake(&request, &[], None).text
}

/// Bug hunt of 3.11.2026, game 3, turn 57: Cave-In's damage to each
/// creature and each player was heard as more entries than the cap but
/// written as fewer lines, and the cap dropped entries from the lines:
/// `drain(..23)` on 21 lines aborted the seat.
#[test]
fn entries_folded_below_the_cap_are_all_told() {
    let text = told(&discards(63, |_| "Treasure".into()));
    assert!(text.contains("P2 discarded Treasure (63 times)."), "{text}");
    assert!(!text.contains("earlier lines not shown"), "{text}");
}

/// Lines past the cap are dropped from the oldest, and counted.
#[test]
fn lines_past_the_cap_drop_the_oldest() {
    let lines = u32::try_from(LOG_LINES).expect("a small cap") + 5;
    let text = told(&discards(lines, |n| format!("Relic {n}")));
    assert!(text.contains("(5 earlier lines not shown)"), "{text}");
    assert!(!text.contains("discarded Relic 4."), "{text}");
    assert!(text.contains("discarded Relic 5."), "{text}");
    assert!(
        text.contains(&format!("discarded Relic {}.", lines - 1)),
        "{text}"
    );
}
