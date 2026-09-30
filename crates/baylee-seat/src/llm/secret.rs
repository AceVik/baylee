//! An API key, held so that nothing prints it.
//!
//! The key is read from the environment once, kept in a [`Secret`], and
//! read back only where a request header is built. Every text that leaves
//! the mind (an error, a transcript line, the terminal) goes through
//! [`scrub`] first, which blanks the key and anything shaped like one,
//! because a provider or a proxy may echo a header back in an error body.

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
/// `x-api-key` up to the next space or quote.
#[must_use]
pub fn scrub(text: &str, secret: Option<&Secret>) -> String {
    let mut out = match secret {
        Some(secret) if secret.0.len() >= 4 => text.replace(&secret.0, "[redacted]"),
        _ => text.to_string(),
    };
    out = blank_after(&out, "sk-", 16);
    for marker in ["Bearer ", "bearer ", "x-api-key: ", "x-api-key\":\""] {
        out = blank_after(&out, marker, 1);
    }
    out
}

/// Blanks each run of key characters that follows `marker`, when the run is
/// at least `least` long. The marker itself stays, so a reader still sees
/// what was there.
fn blank_after(text: &str, marker: &str, least: usize) -> String {
    let key_char = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.';
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(marker) {
        let (before, after) = rest.split_at(at);
        out.push_str(before);
        out.push_str(marker);
        let tail = &after[marker.len()..];
        let run = tail
            .char_indices()
            .find(|(_, c)| !key_char(*c))
            .map_or(tail.len(), |(i, _)| i);
        if run >= least {
            out.push_str("[redacted]");
        } else {
            out.push_str(&tail[..run]);
        }
        rest = &tail[run..];
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
}
