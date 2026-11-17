//! Bounded execution identities shared by hosts and the process agent.
//!
//! These are compatibility contracts, not attestations of an operator's honesty.
//! They contain neither executable paths nor instructions to download or run code.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

/// Maximum size of an executable's one-shot JSON metadata response.
pub const MAX_METADATA_BYTES: usize = 4096;
/// Maximum number of locally installed compatible artifacts advertised by one agent.
pub const MAX_CAPABILITIES: usize = 16;
/// Revision of the execution contract, independent of the websocket protocol.
pub const SCHEMA: u32 = 1;

/// A separately built engine artifact; never selected within the rules hot path.
/// @openapi
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
/// @openapi
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
    /// The native execution contract of a source revision and its AI generation.
    /// This only constructs metadata; callers still choose a statically built artifact.
    #[must_use]
    pub fn native(variant: Variant, build: &str, revision: &str, ai_generation: u32) -> Self {
        let features = match variant {
            Variant::Standard => "kernel=2;loops=off;pack=standard;fork=on",
            Variant::StandardLoops => "kernel=2;loops=on;pack=standard;fork=on",
            Variant::Tower => "kernel=2;loops=off;pack=tower;fork=on",
        };
        let pack = if variant == Variant::Tower {
            "tower"
        } else {
            "standard"
        };
        let mut fingerprint = String::with_capacity(64);
        for byte in Sha256::digest(features.as_bytes()) {
            write!(&mut fingerprint, "{byte:02x}").expect("writing into a string");
        }
        Self {
            schema: SCHEMA,
            protocol: crate::PROTOCOL_VERSION,
            variant,
            build: build.into(),
            features: fingerprint,
            rules: format!("{pack}-{}", if variant == Variant::Tower { 6 } else { 2 }),
            cards: format!("registry@{revision}"),
            ai: format!("{pack}-house-{ai_generation}"),
        }
    }

    /// Writes the identity on the control link, never a local executable path.
    ///
    /// # Panics
    /// Only if serializing this string-and-integer structure into memory fails.
    #[must_use]
    pub fn to_metadata(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("execution metadata serializes")
    }

    /// Rejects malformed, oversized and incompatible metadata before it is advertised.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.protocol == crate::PROTOCOL_VERSION && self.well_formed()
    }

    /// The shape and bounds of [`Self::valid`] without the claim that this is
    /// the executable speaking now. A kept record names the artifact that
    /// played it, and that artifact's transport protocol is history: a reader
    /// of records asks this, or every record stops replaying at the next
    /// protocol revision.
    #[must_use]
    pub fn well_formed(&self) -> bool {
        self.schema == SCHEMA
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

/// Reads a bounded list from an agent's opening frame, refusing partial lists.
#[must_use]
pub fn capabilities_from_metadata(values: &[Vec<u8>]) -> Option<Vec<Identity>> {
    if values.is_empty() || values.len() > MAX_CAPABILITIES {
        return None;
    }
    let identities: Vec<_> = values
        .iter()
        .map(|v| Identity::from_metadata(v))
        .collect::<Option<_>>()?;
    valid_capabilities(&identities).then_some(identities)
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

    #[test]
    fn a_past_protocol_revision_is_well_formed_but_not_current() {
        let mut past = identity();
        past.protocol -= 1;
        assert!(past.well_formed());
        assert!(!past.valid());
        // Shape still binds a record's identity.
        past.features.push('a');
        assert!(!past.well_formed());
        let mut other_schema = identity();
        other_schema.schema += 1;
        assert!(!other_schema.well_formed());
    }
}
