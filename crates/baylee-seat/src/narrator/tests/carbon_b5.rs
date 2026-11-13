//! What the Carbon seat was told wrong in block 5 (12.11.2026), on a real
//! engine: the question that taps permanents to help pay a waterbend was
//! worded as convoke.

use super::*;
use crate::wake::tests::played::position;
use baylee_engine::choice::{PlayerAction, TargetPrompt};
use baylee_gamehost::Session;
use baylee_view::LogTail;

/// What seat 0 is told now, from the view the bridge builds (with the
/// context of what is being paid for).
fn told(session: &Session) -> String {
    let (pending, view) = session.view_for_with_context(ME).expect("seat 0 is asked");
    let request = request(view, pending, LogTail::default());
    Narrator::new(&request.context)
        .wake(&request, &[], None)
        .text
}

fn question(text: &str) -> &str {
    text.lines()
        .find(|line| line.starts_with("QUESTION:"))
        .unwrap_or_else(|| panic!("a question is asked: {text}"))
}

/// Carbon b5, attempt 45, q221: Spirit Water Revival cast with its
/// waterbend {6} paid and two Fog Banks to tap. A waterbend taps artifacts
/// and creatures, each paying {1} of the waterbend cost alone (CR 701.67a,
/// 701.67b); the seat was told "Convoke: choose creatures to tap … (each
/// pays {1} or one mana of its colour)". The engine asks convoke, improvise
/// and a paid waterbend with one prompt, so the question names none of them
/// and says what a tap pays under each.
#[test]
fn a_waterbend_is_not_told_as_convoke() {
    let (mut session, view, _) = position(
        &["Island", "Island", "Island", "Fog Bank", "Fog Bank"],
        &["Spirit Water Revival"],
        &[],
    );
    let card = view.hand[0].id;
    session
        .act_slice(ME, PlayerAction::CastSpell { card }, 0)
        .expect("the Revival is castable");
    session
        .act_slice(ME, PlayerAction::YesNo(true), 0)
        .expect("the waterbend may be paid");
    assert!(
        matches!(
            session.pending(),
            Pending::ChooseTargets {
                reason: TargetPrompt::Convoke,
                ..
            }
        ),
        "the taps are asked: {:?}",
        session.pending()
    );
    let text = told(&session);
    let line = question(&text);
    assert!(
        line.starts_with("QUESTION: Tap permanents to help pay for Spirit Water Revival"),
        "{line}"
    );
    assert!(!line.contains("Convoke:"), "{line}");
    assert!(
        line.contains("waterbend: an artifact or creature pays {1} of the waterbend cost only"),
        "{line}"
    );
}
