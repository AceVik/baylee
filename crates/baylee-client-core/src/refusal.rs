//! A server's refusal in this client's own words (`docs/protocol.md`
//! §"Refusals").
//!
//! A server may be hostile, so it chooses a reason and never a sentence: a
//! known reason reads as this client's text, and the only action offered is
//! one built in. What a server adds is quoted as that server's, as one line of
//! plain text, and nothing in a refusal is ever followed as a link.

use crate::i18n::{Lang, Phrase};
use baylee_protocol::refusal::{Reason, Refusal};

/// What the player reads for `refusal`, which came through
/// [`Refusal::read`] or [`Refusal::cleaned`].
#[must_use]
pub fn say(refusal: &Refusal, lang: Lang) -> String {
    let own = match refusal.reason() {
        Reason::UpdateRequired => refusal.min_version.as_deref().map_or_else(
            || Phrase::RefusalUpdateRequired.text(lang).to_string(),
            |version| Phrase::RefusalUpdateRequiredTo.fill(lang, &[version]),
        ),
        Reason::Maintenance => Phrase::RefusalMaintenance.text(lang).to_string(),
        Reason::Banned => Phrase::RefusalBanned.text(lang).to_string(),
        Reason::Terms => Phrase::RefusalTerms.text(lang).to_string(),
        Reason::SessionEnded => Phrase::RefusalSessionEnded.text(lang).to_string(),
        Reason::Busy => Phrase::RefusalBusy.text(lang).to_string(),
        Reason::Other => Phrase::RefusalOther.text(lang).to_string(),
    };
    match refusal.server_text() {
        Some(text) => Phrase::RefusalServerSaid.fill(lang, &[&own, text]),
        None => own,
    }
}

/// The refusal a socket closed with: its close code and reason, as cleaned as
/// any other. Text does not travel this way, so the code is all there is.
#[must_use]
pub fn of_close(code: u16, reason: &str) -> Option<Refusal> {
    (code == baylee_protocol::refusal::REFUSAL_CLOSE)
        .then(|| Refusal::new(reason).cleaned())
        .flatten()
}

/// The socket library's events carry no close frame, so a refusal rides in the
/// text of its error event. These two are the only readers of that spelling.
const SOCKET_ERROR: &str = "refused:";

/// The text of the error event a socket closed by `refusal` raises.
#[must_use]
pub fn socket_error(refusal: &Refusal) -> String {
    format!("{SOCKET_ERROR}{}", refusal.code)
}

/// The refusal in a socket's error event, cleaned; `None` for any other error.
#[must_use]
pub fn of_socket_error(text: &str) -> Option<Refusal> {
    Refusal::new(text.strip_prefix(SOCKET_ERROR)?).cleaned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_protocol::refusal::REFUSAL_CLOSE;

    #[test]
    fn a_refusal_survives_the_socket_librarys_error_event_and_nothing_else_is_one() {
        let error = socket_error(&Refusal::new("banned"));
        assert_eq!(of_socket_error(&error).map(|r| r.code), Some("banned".into()));
        assert_eq!(of_socket_error("connection reset"), None);
        assert_eq!(of_socket_error("refused:not a code"), None);
    }

    fn with_text(code: &str, text: &str) -> Refusal {
        Refusal {
            code: code.into(),
            text: Some(text.into()),
            ..Refusal::default()
        }
        .cleaned()
        .unwrap()
    }

    #[test]
    fn every_known_code_reads_in_both_languages_and_says_something_of_its_own() {
        for lang in [Lang::En, Lang::De] {
            let said: Vec<String> = [
                "update_required",
                "maintenance",
                "banned",
                "terms",
                "session_ended",
                "busy",
                "something_new",
            ]
            .iter()
            .map(|code| say(&Refusal::new(code), lang))
            .collect();
            for (at, sentence) in said.iter().enumerate() {
                assert!(!sentence.is_empty());
                assert!(!said[..at].contains(sentence), "{sentence}");
            }
        }
    }

    #[test]
    fn the_lowest_version_is_named_when_the_server_gave_one() {
        let refusal = Refusal {
            min_version: Some("0.1.0-beta.10".into()),
            ..Refusal::update_required()
        };
        assert!(say(&refusal, Lang::En).contains("0.1.0-beta.10"));
    }

    #[test]
    fn a_code_this_client_does_not_know_still_says_what_it_can() {
        let bare = say(&Refusal::new("region_closed"), Lang::En);
        assert!(!bare.contains("region_closed"), "a code is not a sentence");
        let with = say(&with_text("region_closed", "Closed in your region."), Lang::En);
        assert!(with.contains("Closed in your region."));
        assert!(with.starts_with(&bare), "its own words come first");
    }

    #[test]
    fn a_hostile_gateway_cannot_put_words_in_the_update_or_terms_sentence() {
        for code in ["update_required", "terms"] {
            let phish = with_text(code, "Download it from evil.example");
            let said = say(&phish, Lang::En);
            assert!(!said.contains("evil.example"), "{said}");
            assert_eq!(said, say(&Refusal::new(code), Lang::En));
        }
    }

    #[test]
    fn a_servers_words_are_kept_apart_from_this_clients() {
        let said = say(&with_text("maintenance", "Back at nine."), Lang::En);
        let own = say(&Refusal::new("maintenance"), Lang::En);
        assert!(said.starts_with(&own));
        assert!(said.contains("Back at nine."));
        assert_ne!(said, format!("{own} Back at nine."), "it is labelled");
    }

    #[test]
    fn only_the_refusal_close_code_is_a_refusal_and_its_reason_is_cleaned() {
        assert_eq!(
            of_close(REFUSAL_CLOSE, "banned").map(|r| r.code),
            Some("banned".to_owned())
        );
        assert_eq!(of_close(1000, "banned"), None);
        assert_eq!(of_close(REFUSAL_CLOSE, "not a code"), None);
        assert_eq!(of_close(REFUSAL_CLOSE, ""), None);
    }
}
