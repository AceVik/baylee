//! An API key, held so that nothing prints it.
//!
//! The key is read from the environment once, kept in a [`Secret`], and
//! read back only where a request header is built. Every text that leaves
//! the mind (an error, a transcript line, the terminal) goes through
//! [`scrub`] first, which blanks the key and anything shaped like one,
//! because a provider or a proxy may echo a header back in an error body.

use baylee_client_core::llmseat::blank_key_shapes;

/// An API key. `Debug` and `Display` say `[redacted]`.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// A key, or `None` for an empty or blank value.
    #[must_use]
    pub fn new(value: &str) -> Option<Self> {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_string()))
    }

    /// The key, for the one header that carries it.
    #[must_use]
    pub(super) fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

impl std::fmt::Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

/// `text` with `secret` and anything shaped like an API key blanked:
/// `sk-` followed by sixteen or more key characters (Anthropic's `sk-ant-…`,
/// `OpenAI`'s and `DeepSeek`'s `sk-…`), and whatever follows `Bearer ` or
/// `x-api-key` up to the next space or quote. The shapes are the ones the
/// settings file is refused for ([`blank_key_shapes`]); and the
/// credentials of any address that carries them (`https://user:pass@host`,
/// [`blank_userinfo`]), which an endpoint's address may and an error that
/// names it then would.
#[must_use]
pub fn scrub(text: &str, secret: Option<&Secret>) -> String {
    let out = match secret {
        Some(secret) if secret.0.len() >= 4 => text.replace(&secret.0, "[redacted]"),
        _ => text.to_string(),
    };
    blank_key_shapes(&blank_userinfo(&out))
}

/// `text` with the user and password of every address in it blanked: what
/// stands between `://` and an `@` before the address's host ends (`/`,
/// `?`, `#`, a space, a quote, a closing bracket or the end).
#[must_use]
pub fn blank_userinfo(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("://") {
        let (head, tail) = rest.split_at(at + 3);
        out.push_str(head);
        let end = tail
            .find(|c: char| {
                matches!(c, '/' | '?' | '#' | '\'' | '"' | ')' | '>' | ']') || c.is_whitespace()
            })
            .unwrap_or(tail.len());
        let authority = &tail[..end];
        match authority.rfind('@') {
            Some(cut) => {
                out.push_str("[redacted]");
                out.push_str(&authority[cut..]);
            }
            None => out.push_str(authority),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_never_prints_and_scrub_blanks_what_looks_like_a_key() {
        let key = Secret::new("TEST-key-0123456789abcdef").expect("a key");
        assert_eq!(format!("{key:?} {key}"), "[redacted] [redacted]");
        let echoed = "401: invalid x-api-key: TEST-key-0123456789abcdef (Bearer abc.def)";
        let clean = scrub(echoed, Some(&key));
        assert!(!clean.contains("0123456789abcdef"), "{clean}");
        assert!(!clean.contains("abc.def"), "{clean}");
        // Shaped like a key, though not this one.
        let other = scrub("key sk-ant-api03-AAAABBBBCCCCDDDD-x ok", None);
        assert_eq!(other, "key sk-[redacted] ok");
        // Short runs after `sk-` are words, not keys.
        assert_eq!(scrub("a task-list", None), "a task-list");
        assert!(Secret::new("  ").is_none());
    }

    /// An address's credentials never reach a sentence (the chair card's
    /// line, a log): blanked wherever an address carries them, the rest of
    /// the sentence and every other address as it was.
    #[test]
    fn an_addresses_credentials_are_blanked() {
        let said = "the provider could not be reached: https://alice:s3cret-pass@llm.example:8443/v1/models: \
                    connection refused (see http://example.org/help)";
        let clean = scrub(said, None);
        assert!(!clean.contains("s3cret-pass"), "{clean}");
        assert!(!clean.contains("alice"), "{clean}");
        assert!(
            clean.contains("https://[redacted]@llm.example:8443/v1/models"),
            "{clean}"
        );
        assert!(clean.contains("http://example.org/help"), "{clean}");
        assert_eq!(blank_userinfo("no address here"), "no address here");
        assert_eq!(blank_userinfo("http://token@h"), "http://[redacted]@h");
        assert_eq!(
            blank_userinfo("'http://a:b@h' and ://"),
            "'http://[redacted]@h' and ://"
        );
    }
}
