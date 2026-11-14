//! Bounded execution identities shared by hosts and the process agent.
//!
//! These are compatibility contracts, not attestations of an operator's honesty.
//! They contain neither executable paths nor instructions to download or run code.
use serde::{Deserialize, Serialize};

/// Maximum size of an executable's one-shot JSON metadata response.
pub const MAX_METADATA_BYTES: usize = 4096;
/// Maximum number of locally installed compatible artifacts advertised by one agent.
pub const MAX_CAPABILITIES: usize = 16;
/// Revision of the execution contract, independent of the websocket protocol.
pub const SCHEMA: u32 = 1;

/// A separately built engine artifact; never selected within the rules hot path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Variant {
    /// Ordinary tables with no repetition detector.
    Standard,
    /// Ordinary tables with the explicit break/draw detector.
    StandardLoops,
    /// Tower's native rules and AI, with no repetition detector.
    Tower,
}

/// The exact executable capabilities frozen by a game reservation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    /// Contract schema.
    pub schema: u32,
    /// Transport protocol spoken by this executable.
    pub protocol: u32,
    /// The statically selected engine variant.
    pub variant: Variant,
    /// Source/build identity; identical source with different features still differs below.
    pub build: String,
    /// Lowercase SHA-256 of the executable's canonical compile-time feature description.
    pub features: String,
    /// Native rule-pack identity, including its revision.
    pub rules: String,
    /// Card-registry identity.
    pub cards: String,
    /// Deterministic AI policy identity.
    pub ai: String,
}

fn label(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.trim() == value
        && value.bytes().all(|byte| (32..=126).contains(&byte))
}

impl Identity {
    /// Rejects malformed, oversized and incompatible metadata before it is advertised.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.schema == SCHEMA
            && self.protocol == crate::PROTOCOL_VERSION
            && label(&self.build, 160)
            && self.features.len() == 64
            && self
                .features
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && [&self.rules, &self.cards, &self.ai]
                .into_iter()
                .all(|s| label(s, 96))
    }

    /// Reads one bounded metadata document. Extra fields and trailing documents fail.
    #[must_use]
    pub fn from_metadata(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_METADATA_BYTES {
            return None;
        }
        let value: Self = serde_json::from_slice(bytes).ok()?;
        value.valid().then_some(value)
    }
}

/// A bounded capability set with no duplicate identities.
#[must_use]
pub fn valid_capabilities(values: &[Identity]) -> bool {
    !values.is_empty()
        && values.len() <= MAX_CAPABILITIES
        && values.iter().all(Identity::valid)
        && values
            .iter()
            .enumerate()
            .all(|(at, value)| !values[..at].contains(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> Identity {
        Identity {
            schema: SCHEMA,
            protocol: crate::PROTOCOL_VERSION,
            variant: Variant::Standard,
            build: "test-build".into(),
            features: "a".repeat(64),
            rules: "standard-2".into(),
            cards: "registry@test".into(),
            ai: "house@test".into(),
        }
    }

    #[test]
    fn metadata_is_bounded_strict_and_has_no_remote_execution_fields() {
        let encoded = serde_json::to_vec(&identity()).unwrap();
        assert_eq!(Identity::from_metadata(&encoded), Some(identity()));
        let mut object = serde_json::to_value(identity()).unwrap();
        object["executable"] = "/tmp/untrusted".into();
        assert!(Identity::from_metadata(&serde_json::to_vec(&object).unwrap()).is_none());
        assert!(Identity::from_metadata(&[b' '; MAX_METADATA_BYTES + 1]).is_none());
        assert!(Identity::from_metadata(&[encoded.as_slice(), b"{}"].concat()).is_none());
        assert!(Identity::from_metadata(b"\xff").is_none());
    }

    #[test]
    fn capability_admission_rejects_skew_controls_and_duplicates() {
        let valid = identity();
        assert!(valid_capabilities(std::slice::from_ref(&valid)));
        assert!(!valid_capabilities(&[]));
        assert!(!valid_capabilities(&[valid.clone(), valid.clone()]));
        assert!(!valid_capabilities(&vec![
            valid.clone();
            MAX_CAPABILITIES + 1
        ]));
        let mut invalid = valid.clone();
        invalid.protocol += 1;
        assert!(!invalid.valid());
        invalid = valid.clone();
        invalid.features.replace_range(..1, "A");
        assert!(!invalid.valid());
        invalid = valid.clone();
        invalid.build.push('\n');
        assert!(!invalid.valid());
        invalid = valid;
        invalid.cards = "x".repeat(97);
        assert!(!invalid.valid());
    }
}
