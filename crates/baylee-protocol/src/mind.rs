//! What answers a seat, as its client declares it (`v1::SeatMind`), and the
//! one predicate for text shaped like a key.
//!
//! A declaration is self-declared and unverified: the engine checks its
//! shape here and writes it into the game's record (#315), and the bridge
//! and the client check what they send by the same rule. The shape is what
//! keeps a secret out of the record: every field is a short word in the
//! characters a model id is written in, so an address (`://`), a key
//! ([`shaped_like_a_key`]) and a prompt (a sentence has spaces) are each
//! refused rather than written down. `docs/protocol.md` §"Who answers a
//! seat, as it says".

use crate::v1::{SeatMind, seat_mind::Kind};

/// The most characters a provider, an effort or a level holds.
pub const WORD_MAX: usize = 32;

/// The most characters a model id holds, as the settings file allows
/// (`llmseat::model_fault`).
pub const MODEL_MAX: usize = 100;

/// Why `mind` is not a declaration the record takes, or `None`.
///
/// The kind must be said; a provider and a model belong to a language
/// model (an API's model is required, a CLI may play its own default), a
/// level to the house, and a person or a script carries none of them. Each
/// field is at most [`WORD_MAX`] characters ([`MODEL_MAX`] for the model) of
/// letters, digits and `-_.:/@`, holds no `://` and nothing
/// [`shaped_like_a_key`].
#[must_use]
pub fn fault(mind: &SeatMind) -> Option<&'static str> {
    let fields = [
        (&mind.provider, WORD_MAX, "provider"),
        (&mind.model, MODEL_MAX, "model"),
        (&mind.effort, WORD_MAX, "effort"),
        (&mind.level, WORD_MAX, "level"),
    ];
    for (text, max, _) in fields {
        if text.chars().count() > max {
            return Some("a declared mind's field is too long");
        }
        if !text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.:/@".contains(c))
        {
            return Some("a declared mind's field holds a character no model id has");
        }
        if text.contains("://") {
            return Some("a declared mind names no address");
        }
        if shaped_like_a_key(text) {
            return Some("a declared mind carries no key");
        }
    }
    let has = |text: &String| !text.is_empty();
    let no_level = !has(&mind.level);
    let nothing = !has(&mind.provider) && !has(&mind.model) && !has(&mind.effort) && no_level;
    match Kind::try_from(mind.kind) {
        Ok(Kind::Unspecified) | Err(_) => Some("a declared mind says what kind it is"),
        Ok(Kind::Human | Kind::Scripted) if !nothing => {
            Some("a person or a script declares no model")
        }
        Ok(Kind::House) if has(&mind.provider) || has(&mind.model) || has(&mind.effort) => {
            Some("the house declares its level and no model")
        }
        Ok(Kind::LlmApi) if !has(&mind.provider) || !has(&mind.model) => {
            Some("a model behind an API declares its provider and its model")
        }
        Ok(Kind::LlmCli) if !has(&mind.provider) => Some("a model behind a CLI declares its tool"),
        Ok(Kind::LlmApi | Kind::LlmCli) if !no_level => {
            Some("a language model declares no house level")
        }
        Ok(_) => None,
    }
}

/// Whether `text` holds anything shaped like a key: a marker with as many
/// key characters after it as such a key has, or an authorization header's
/// words.
///
/// Two kinds of marker. The **generic** ones (`sk-`, `Bearer `, `x-api-key`)
/// are short and common inside words, so they count only where a word
/// starts: glued to a lowercase word before it (`desk-tools-collection`,
/// `risk-free`: a lowercase letter, `_`, `-` or `.` before it) they are part
/// of that word and no key. Anything else before one starts a word: the
/// text's start, a space, `/`, `=`, an uppercase letter, a non-ASCII
/// character, and a digit, since a model id ends in one and a key pasted
/// after it (`claude-sonnet-5-5sk-…`) starts where its marker does.
///
/// The **provider-specific** ones (`sk-ant-`, `sk-proj-`, GitHub's `ghp_`,
/// `gho_`, `ghu_`, `ghs_`, `ghr_`, `github_pat_`) are long and followed by at
/// least twenty key characters, which no word is, so they count wherever
/// they stand: a key pasted onto a model id (`sonnetsk-ant-…`) is still one,
/// while `task-ant-hill` is too short after its marker to be.
///
/// The one definition: the settings file and panel refuse by it
/// (`baylee_client_core::llmseat` re-exports it), the seat bridge refuses
/// to start a CLI whose environment holds such a value, and the engine
/// refuses a declared mind that carries one ([`fault`]).
#[must_use]
pub fn shaped_like_a_key(text: &str) -> bool {
    /// Marker, key characters it needs after it, and whether it must start
    /// a word.
    const MARKERS: [(&str, usize, bool); 12] = [
        ("sk-", 16, true),
        ("sk-ant-", 20, false),
        ("sk-proj-", 20, false),
        ("ghp_", 20, false),
        ("gho_", 20, false),
        ("ghu_", 20, false),
        ("ghs_", 20, false),
        ("ghr_", 20, false),
        ("github_pat_", 20, false),
        ("Bearer ", 1, true),
        ("bearer ", 1, true),
        ("x-api-key", 0, true),
    ];
    MARKERS.iter().any(|(marker, least, word)| {
        text.match_indices(marker).any(|(at, _)| {
            let starts_a_word = text[..at]
                .chars()
                .next_back()
                .is_none_or(|c| !(c.is_ascii_lowercase() || matches!(c, '_' | '-' | '.')));
            let run = text[at + marker.len()..]
                .chars()
                .take_while(|c| key_char(*c))
                .count();
            (starts_a_word || !word) && run >= *least
        })
    })
}

/// A character a key is written in: what [`shaped_like_a_key`] counts after
/// a marker, and what a redaction blanks.
#[must_use]
pub fn key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
}

#[cfg(test)]
mod tests {
    use super::{Kind, SeatMind, fault};

    fn mind(kind: Kind, provider: &str, model: &str, effort: &str, level: &str) -> SeatMind {
        SeatMind {
            kind: kind as i32,
            provider: provider.into(),
            model: model.into(),
            effort: effort.into(),
            level: level.into(),
        }
    }

    /// What the bridge and the client send passes.
    #[test]
    fn every_kind_declared_as_sent_passes() {
        for sent in [
            mind(Kind::Human, "", "", "", ""),
            mind(Kind::Scripted, "", "", "", ""),
            mind(Kind::House, "", "", "", "steady"),
            mind(Kind::LlmApi, "anthropic", "claude-opus-5-5", "high", ""),
            mind(Kind::LlmApi, "openai", "deepseek-chat", "", ""),
            mind(
                Kind::LlmApi,
                "anthropic",
                "claude-opus-5-5@20260101",
                "",
                "",
            ),
            mind(Kind::LlmCli, "claude", "opus", "low", ""),
            mind(Kind::LlmCli, "codex", "", "", ""),
        ] {
            assert_eq!(fault(&sent), None, "{sent:?}");
        }
    }

    /// A key glued to a model, a key as the effort, an address with a
    /// password in it, a prompt, a control character, a bidi override and
    /// an overlong id are each refused, and so is a mind of no kind.
    #[test]
    fn a_key_an_address_a_prompt_or_no_kind_is_refused() {
        let key = format!("sk-ant-{}", "A".repeat(24));
        for refused in [
            mind(
                Kind::LlmApi,
                "anthropic",
                &format!("claude-opus-5-5{key}"),
                "",
                "",
            ),
            mind(Kind::LlmApi, "anthropic", "claude-opus-5-5", &key, ""),
            mind(
                Kind::LlmApi,
                "openai",
                &format!("ghp_{}", "x".repeat(24)),
                "",
                "",
            ),
            mind(Kind::LlmApi, "https://me:pw@llm.example", "m", "", ""),
            mind(Kind::LlmApi, "openai", "http://llm.example/v1", "", ""),
            mind(
                Kind::LlmApi,
                "anthropic",
                "you are a careful player",
                "",
                "",
            ),
            mind(Kind::LlmApi, "anthropic", "claude\u{7}", "", ""),
            mind(Kind::LlmApi, "anthropic", "claude\u{202e}", "", ""),
            mind(Kind::LlmApi, "anthropic", &"m".repeat(101), "", ""),
            mind(Kind::Unspecified, "", "", "", ""),
            SeatMind {
                kind: 99,
                ..SeatMind::default()
            },
        ] {
            assert!(fault(&refused).is_some(), "{refused:?}");
        }
    }

    /// Each kind carries only its own fields.
    #[test]
    fn a_kind_carries_only_its_own_fields() {
        for refused in [
            mind(Kind::Human, "anthropic", "", "", ""),
            mind(Kind::Scripted, "", "", "", "steady"),
            mind(Kind::House, "", "claude-opus-5-5", "", "steady"),
            mind(Kind::LlmApi, "anthropic", "", "", ""),
            mind(Kind::LlmApi, "", "claude-opus-5-5", "", ""),
            mind(Kind::LlmCli, "", "opus", "", ""),
            mind(Kind::LlmCli, "claude", "opus", "", "steady"),
        ] {
            assert!(fault(&refused).is_some(), "{refused:?}");
        }
        assert_eq!(
            fault(&mind(Kind::LlmApi, "anthropic", &"m".repeat(100), "", "")),
            None
        );
    }
}
