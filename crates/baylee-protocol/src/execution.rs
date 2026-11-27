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

    /// Compatibility key for two independently running ranked engines. Unstamped
    /// and dirty builds can share a version label despite having different rules.
    /// Only a clean, source-identified competitive engine can offer this key.
    #[must_use]
    pub fn ranked_compatibility(&self) -> Option<[u8; 32]> {
        let revision = self.cards.strip_prefix("registry@")?;
        let (_, stamp) = self.build.rsplit_once(" (")?;
        let short = stamp.strip_suffix(')')?;
        if !self.valid()
            || self.variant != Variant::StandardLoops
            || !matches!(revision.len(), 40 | 64)
            || !revision
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || short.len() < 10
            || !revision.starts_with(short)
        {
            return None;
        }
        let mut digest = Sha256::new();
        digest.update(b"baylee/ranked/engine-compatibility/v1\0");
        digest.update(self.to_metadata());
        Some(digest.finalize().into())
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

    /// Whether an agent and a gateway of this build may still run a game on
    /// this artifact: well formed, and of this build's protocol or an older
    /// one. A game goes on on the build that started it after an update
    /// (`docs/proposals/games-survive-restarts.md`), so the agent installs
    /// older builds beside the current one and the gateway takes them; it
    /// orders them only for a game an older build played ([`Self::valid`] is
    /// what a new game, a ranked duel, a challenge and Tower ask). A newer
    /// protocol is refused: nothing here can know what it says.
    #[must_use]
    pub fn hostable(&self) -> bool {
        (1..=crate::PROTOCOL_VERSION).contains(&self.protocol) && self.well_formed()
    }

    /// Reads one bounded metadata document. Extra fields and trailing documents fail.
    #[must_use]
    pub fn from_metadata(bytes: &[u8]) -> Option<Self> {
        Self::read_metadata(bytes).filter(Self::valid)
    }

    /// [`Self::from_metadata`] for an artifact of this build or an older one
    /// ([`Self::hostable`]).
    #[must_use]
    pub fn from_hostable_metadata(bytes: &[u8]) -> Option<Self> {
        Self::read_metadata(bytes).filter(Self::hostable)
    }

    fn read_metadata(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_METADATA_BYTES {
            return None;
        }
        serde_json::from_slice(bytes).ok()
    }
}

/// Reads a bounded list from an agent's opening frame, refusing partial lists.
/// Older builds an agent keeps installed are in it ([`Identity::hostable`]);
/// whoever orders an engine picks what it needs.
#[must_use]
pub fn capabilities_from_metadata(values: &[Vec<u8>]) -> Option<Vec<Identity>> {
    if values.is_empty() || values.len() > MAX_CAPABILITIES {
        return None;
    }
    let identities: Vec<_> = values
        .iter()
        .map(|v| Identity::from_hostable_metadata(v))
        .collect::<Option<_>>()?;
    hostable_capabilities(&identities).then_some(identities)
}

/// A bounded capability set of this build's protocol, with no duplicate
/// identities.
#[must_use]
pub fn valid_capabilities(values: &[Identity]) -> bool {
    bounded_and_distinct(values) && values.iter().all(Identity::valid)
}

/// [`valid_capabilities`] for a set that may hold older builds
/// ([`Identity::hostable`]).
#[must_use]
pub fn hostable_capabilities(values: &[Identity]) -> bool {
    bounded_and_distinct(values) && values.iter().all(Identity::hostable)
}

fn bounded_and_distinct(values: &[Identity]) -> bool {
    !values.is_empty()
        && values.len() <= MAX_CAPABILITIES
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
    fn ranked_peers_require_a_stamped_build_and_the_same_rules_and_registry() {
        let revision = "d102ca933e0ec8827d7579ec57cbe3f88d7349ad";
        let clean = Identity::native(Variant::StandardLoops, "test (d102ca933e)", revision, 32);
        let key = clean.ranked_compatibility().unwrap();
        let round_trip = Identity::from_metadata(&clean.to_metadata()).unwrap();
        assert_eq!(round_trip.ranked_compatibility(), Some(key));

        let mut other = clean.clone();
        other.cards = format!("registry@{}00", &revision[..38]);
        assert_ne!(other.ranked_compatibility().unwrap(), key);
        for (field, changed) in [
            ("build", "different (d102ca933e)"),
            ("rules", "standard-3"),
            ("ai", "standard-house-33"),
            ("features", &"f".repeat(64)),
        ] {
            let mut value = serde_json::to_value(&clean).unwrap();
            value[field] = changed.into();
            let other: Identity = serde_json::from_value(value).unwrap();
            assert_ne!(other.ranked_compatibility().unwrap(), key, "{field}");
        }
        for stamp in [
            "unidentified",
            "test (d102ca933e-dirty)",
            "test (a102ca933e)",
        ] {
            other.build = stamp.into();
            assert_eq!(other.ranked_compatibility(), None);
        }
        assert!(
            Identity::native(Variant::Standard, &clean.build, revision, 32)
                .ranked_compatibility()
                .is_none()
        );
        assert!(
            Identity::native(Variant::StandardLoops, &clean.build, "unknown", 32)
                .ranked_compatibility()
                .is_none()
        );
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

    /// An older build an agent keeps is taken beside the current one and read
    /// from its metadata; a newer one, one that names no protocol and a
    /// malformed one are not.
    #[test]
    fn an_older_build_is_hostable_and_a_newer_one_is_not() {
        let current = identity();
        let mut older = identity();
        older.protocol -= 1;
        older.build = "older-build".into();
        assert!(current.hostable() && older.hostable());
        assert!(!older.valid());
        assert_eq!(
            Identity::from_hostable_metadata(&older.to_metadata()),
            Some(older.clone())
        );
        assert_eq!(Identity::from_metadata(&older.to_metadata()), None);
        let both = [current.clone(), older.clone()];
        assert!(hostable_capabilities(&both));
        assert!(!valid_capabilities(&both));
        assert_eq!(
            capabilities_from_metadata(&both.iter().map(Identity::to_metadata).collect::<Vec<_>>()),
            Some(both.to_vec())
        );
        for protocol in [0, crate::PROTOCOL_VERSION + 1] {
            let mut other = current.clone();
            other.protocol = protocol;
            assert!(!other.hostable(), "{protocol}");
            assert_eq!(Identity::from_hostable_metadata(&other.to_metadata()), None);
        }
        older.features.push('a');
        assert!(!older.hostable());
    }
}
