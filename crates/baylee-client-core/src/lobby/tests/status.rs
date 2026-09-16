//! The one line the lobby writes about itself: which phrase stands there, which of the two tones carries it, and that the language is read when a sentence is made rather than re-read afterwards — a status line is a record of what just happened, not a label that keeps re-rendering. A refusal is the only thing on it a player has to act on, so every test here is about the two staying apart: an empty line, a form that was refused, a gateway saying no, and a finished duel that must not be drawn in the colour that means somebody is waited for. What each of those events does to the seat or the screen is asserted where that change lives; only its reading is here.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// The lobby's own sentences are drawn from the phrase table, so setting
/// the language changes what the *next* one says. Nothing re-translates a
/// line already on screen, which is the point: a status line is a record
/// of what just happened, not a label that keeps re-rendering.
#[test]
fn a_status_line_is_said_in_the_lobbys_language() {
    let mut lobby = seated_lobby();
    lobby.apply(LobbyEvent::Decks(vec![]));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert_eq!(lobby.host(GameMode::Ai), None);
    assert_eq!(lobby.status(), "pick a deck first");

    lobby.set_lang(Lang::De);
    assert_eq!(lobby.lang(), Lang::De);
    assert_eq!(lobby.host(GameMode::Ai), None);
    assert_eq!(lobby.status(), "wähle zuerst ein Deck");
}

/// The shell says things too, and has no business knowing the language.
#[test]
fn the_shell_says_its_own_sentences_in_that_language_too() {
    let mut lobby = seated_lobby();
    lobby.set_lang(Lang::De);
    lobby.tell(Phrase::CouldNotReachTable, &["Zeitüberschreitung"]);
    assert_eq!(lobby.status(), "Tisch nicht erreichbar: Zeitüberschreitung");
}

/// A game that ended is not a refusal, and a table that could not be
/// reached is.
///
/// Both leave the seat by the same door, and the words alone cannot tell
/// them apart — `Tone` is the whole of it, and the lobby draws a refusal
/// in red. Every finished duel used to end with its own ending written
/// up there in the colour that means somebody has to do something.
#[test]
fn the_end_of_a_game_is_read_differently_from_a_table_that_refused_one() {
    let mut lobby = seated_lobby();
    lobby.stand_up(Phrase::GameEnded, &[]);
    assert_eq!(*lobby.screen(), Screen::Table, "and still leaves the seat");
    assert_eq!(lobby.tone(), Tone::Note);

    lobby.unseat_because(Phrase::CouldNotReachTable, &["no route"]);
    assert_eq!(lobby.tone(), Tone::Refusal);
}

#[test]
fn a_refusal_reads_differently_from_a_note() {
    let mut lobby = Lobby::new();
    assert_eq!(lobby.tone(), Tone::Note, "an empty line is no refusal");
    assert!(lobby.submit().is_none());
    assert_eq!(
        lobby.tone(),
        Tone::Refusal,
        "a form with no address in it is a form that was refused"
    );
    lobby.set_field(Field::Email, "a@b.c");
    lobby.set_field(Field::Password, "pw");
    assert!(lobby.submit().is_some());
    assert_eq!(
        lobby.tone(),
        Tone::Note,
        "and signing in is the lobby getting on with it"
    );
    lobby.apply(LobbyEvent::Failed("wrong password".to_string()));
    assert_eq!(
        lobby.tone(),
        Tone::Refusal,
        "the gateway saying no is the plainest refusal there is"
    );
}

#[test]
fn a_failure_is_shown_and_nothing_else() {
    let mut lobby = seated_lobby();
    lobby.host(GameMode::Ai);
    assert_eq!(
        lobby.apply(LobbyEvent::Failed("no such deck".to_string())),
        None
    );
    assert_eq!(lobby.status(), "no such deck");
    assert_eq!(*lobby.screen(), Screen::Table, "we stay where we were");
    assert!(!lobby.busy(), "and the lobby is usable again");
}
