//! Localized, known server messages. Unknown diagnostics remain intact.
use super::{Lang, ServerPhrase};

/// Translate server-owned status keys without changing card or player names.
///
/// The words are `locales/<lang>/server.ftl`: a message there is matched by
/// its English text exactly, and one the client does not know is shown as
/// it came, so a newer server stays diagnosable by an older client.
#[must_use]
pub fn server_message(lang: Lang, message: &str) -> String {
    if lang == Lang::En {
        return message.to_owned();
    }
    let detail = message.strip_prefix("casting: ").unwrap_or(message);
    let detail = detail.strip_prefix("illegal action: ").unwrap_or(detail);
    known(detail).map_or_else(|| message.to_owned(), |phrase| phrase.text(lang).to_owned())
}

/// The server message whose English is `detail`.
fn known(detail: &str) -> Option<ServerPhrase> {
    ServerPhrase::ALL
        .iter()
        .copied()
        .find(|phrase| phrase.text(Lang::En) == detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_errors_translate_without_dropping_unknown_diagnostics() {
        for phrase in ServerPhrase::ALL {
            let english = phrase.text(Lang::En);
            let german = phrase.text(Lang::De);
            assert!(!german.is_empty());
            assert_ne!(english, german, "{phrase:?} has no German");
            assert_eq!(server_message(Lang::En, english), english);
            assert_eq!(server_message(Lang::De, english), german);
        }
        assert_eq!(
            server_message(Lang::De, "casting: illegal action: not enough mana"),
            "Es ist nicht genug Mana vorhanden."
        );
        assert_eq!(
            server_message(Lang::De, "unrecognized server diagnostic 42"),
            "unrecognized server diagnostic 42"
        );
    }
}
