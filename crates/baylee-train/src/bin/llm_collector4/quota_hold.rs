//! Operational provider exhaustion evidence, independent of game admission.
//! Never persist the provider's raw error (or prompt/transcript contents).
use std::io::BufRead as _;
use std::path::Path;

use anyhow::{Context as _, ensure};
use serde_json::{Value, json};

use crate::audit::{sha256_bytes, write_atomic_json};

pub const HOLD_FILE: &str = "quota-hold.json";
const MAX_RESET_SECONDS: u64 = 7 * 24 * 3600;

fn reset_seconds(error: &str) -> Option<u64> {
    if error.matches("Resets in ").count() != 1 {
        return None;
    }
    let (_, tail) = error.split_once("Resets in ")?;
    let token = tail.split_whitespace().next()?.trim_end_matches('.');
    let mut rest = token;
    let mut total = 0_u64;
    let mut previous = 4;
    while !rest.is_empty() {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        let count = rest[..digits].parse::<u64>().ok()?;
        let unit = *rest.as_bytes().get(digits)?;
        let (rank, factor) = match unit {
            b'h' => (3, 3600),
            b'm' => (2, 60),
            b's' => (1, 1),
            _ => return None,
        };
        if rank >= previous {
            return None;
        }
        previous = rank;
        total = total.checked_add(count.checked_mul(factor)?)?;
        rest = &rest[digits + 1..];
    }
    (total > 0 && total <= MAX_RESET_SECONDS).then_some(total)
}

/// The words a CLI's own exhaustion message opens with: `agy`'s individual
/// quota, Codex's usage limit (`Try again at …` or a plan hint follows).
const NATIVE_EXHAUSTION: [&str; 3] = [
    "Individual quota reached.",
    "You've hit your usage limit.",
    "You\u{2019}ve hit your usage limit.",
];

/// A seat's own exhaustion: the mind failed with it, or a fallback took
/// over from it (`{"fallback": {"cause": "limited", "why": …}}`), so the
/// game went on but the primary is spent for the next ones.
fn explicit_individual_failure(row: &Value) -> Option<&str> {
    let (error, native) = if let Some(fallback) = row.get("fallback") {
        if fallback.get("cause")?.as_str()? != "limited" {
            return None;
        }
        let why = fallback.get("why")?.as_str()?;
        (why, why)
    } else {
        if row.get("event")?.as_str()? != "mind_failed" {
            return None;
        }
        let error = row.get("error")?.as_str()?;
        let native = error
            .strip_prefix("the mind is rate limited: ")
            .or_else(|| error.strip_prefix("the mind is unavailable: rate limit: "))?;
        (error, native)
    };
    // The provider message must declare exhaustion, not merely mention it in
    // arbitrary model/prompt text. Preserve only a digest of this error.
    NATIVE_EXHAUSTION
        .iter()
        .any(|sign| native.starts_with(sign))
        .then_some(error)
}

pub fn inspect_attempt(attempt: &Path, now: f64) -> anyhow::Result<Vec<Value>> {
    let dir = attempt.join("transcripts");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut evidence = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }
        let file = std::fs::File::open(&path)?;
        let modified = file
            .metadata()?
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f64();
        for (index, line) in std::io::BufReader::new(file).lines().enumerate() {
            let line = line?;
            // A truncated final write must not hide earlier complete failures.
            let Ok(row) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let Some(error) = explicit_individual_failure(&row) else {
                continue;
            };
            let seconds = reset_seconds(error);
            let reset = seconds.map(|seconds| {
                modified + f64::from(u32::try_from(seconds).expect("bounded reset seconds"))
            });
            if reset.is_some_and(|reset| reset <= now) {
                continue;
            }
            evidence.push(json!({
                "attempt_id": attempt.file_name().and_then(|s| s.to_str()),
                "transcript_file": path.file_name().and_then(|s| s.to_str()),
                "row_index": index,
                "cause": "explicit_native_individual_quota_exhausted",
                "error_sha256": sha256_bytes(error.as_bytes()),
                "reset_seconds": seconds,
                "reset_not_before_unix": reset,
                "reset_anchor": "transcript_mtime_conservative_upper_bound",
                "requires_reset_review": seconds.is_none(),
            }));
        }
    }
    Ok(evidence)
}

pub fn persist(root: &Path, mut evidence: Vec<Value>, now: f64) -> anyhow::Result<()> {
    let path = root.join(HOLD_FILE);
    if path.exists() {
        let previous: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        let prior = previous["evidence"]
            .as_array()
            .context("Invalid quota hold evidence")?;
        for item in prior {
            if !evidence.contains(item) {
                evidence.push(item.clone());
            }
        }
    }
    ensure!(!evidence.is_empty(), "Quota hold requires evidence");
    let unknown = evidence
        .iter()
        .any(|item| item["reset_not_before_unix"].as_f64().is_none());
    let reset = if unknown {
        None
    } else {
        evidence
            .iter()
            .filter_map(|item| item["reset_not_before_unix"].as_f64())
            .reduce(f64::max)
    };
    write_atomic_json(
        &path,
        &json!({
            "schema_version": 1, "status": "held", "cause": "individual_provider_quota",
            "created_or_updated_unix": now, "reset_not_before_unix": reset,
            "requires_operator_review": true,
            "scope": "Stop new assignments only; preserve active games and unchanged data admission. Hold persists across restart until reviewed and explicitly cleared.",
            "evidence": evidence,
        }),
    )
}

/// Refuse even malformed existing holds. A new process must never silently
/// clear a previous controller's provider hold or guess that its reset passed.
pub fn check_startup(root: &Path, now: f64) -> anyhow::Result<()> {
    ensure!(
        !root.join(HOLD_FILE).exists(),
        "Persistent quota hold exists; review quota-hold.json before resuming"
    );
    let attempts = root.join("attempts");
    if !attempts.exists() {
        return Ok(());
    }
    let mut evidence = Vec::new();
    for attempt in std::fs::read_dir(attempts)? {
        evidence.extend(inspect_attempt(&attempt?.path(), now)?);
    }
    if !evidence.is_empty() {
        persist(root, evidence, now)?;
        anyhow::bail!(
            "Individual provider quota exhausted; persisted quota-hold.json before startup"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn failure(message: &str) -> Value {
        json!({"event":"mind_failed","error":format!("the mind is unavailable: rate limit: {message}")})
    }
    #[test]
    fn only_explicit_native_individual_failures_trigger_hold() {
        for row in [
            failure("Individual quota reached. Resets in 90h56m56s."),
            failure("You've hit your usage limit. Try again at 3:04 PM."),
            json!({"fallback":{"profile":"haiku-medium","turn":3,"cause":"limited",
                "why":"You've hit your usage limit. Try again at 3:04 PM."}}),
            json!({"fallback":{"profile":"sonnet-low","turn":9,"cause":"limited",
                "why":"Individual quota reached. Resets in 30m."}}),
        ] {
            assert!(explicit_individual_failure(&row).is_some(), "{row}");
        }
        for row in [
            failure("busy; cooldown 30s"),
            failure("429 rate limit"),
            failure("provider mentioned Individual quota reached. Resets in 90h"),
            json!({"event":"prompt","error":"the mind is rate limited: Individual quota reached. Resets in 90h"}),
            json!({"event":"mind_failed","error":"Individual quota reached. Resets in 90h"}),
            json!({"fallback":{"profile":"haiku-medium","turn":1,"cause":"failed",
                "why":"You've hit your usage limit."}}),
            json!({"fallback":{"profile":"haiku-medium","turn":1,"cause":"limited",
                "why":"429: no reason given"}}),
            json!({"fallback":{"profile":"haiku-medium","turn":1,"cause":"limited",
                "why":"the model said You've hit your usage limit."}}),
        ] {
            assert!(explicit_individual_failure(&row).is_none());
        }
    }
    #[test]
    fn reset_is_positive_bounded_and_not_guessed() {
        assert_eq!(reset_seconds("Resets in 90h56m56s."), Some(327_416));
        assert_eq!(reset_seconds("Resets in 30m."), Some(1800));
        assert_eq!(reset_seconds("Resets in 1s."), Some(1));
        for text in [
            "Resets in 0s",
            "Resets in 169h",
            "Resets in 1m1h",
            "Resets in -1h",
            "Resets in 99999999999999999999h",
            "Resets in soon",
            "Resets in 1hgarbage",
            "no reset",
            "Resets in 1h. Resets in 4h.",
        ] {
            assert_eq!(reset_seconds(text), None);
        }
    }
    #[test]
    fn failed_partial_transcript_holds_without_changing_admission_and_survives_restart() {
        let temp = crate::audit::tests::TestDir::new("quota-hold");
        let attempt = temp.path().join("attempts/attempt-000000001");
        std::fs::create_dir_all(attempt.join("transcripts")).unwrap();
        let receipt = b"{\"status\":\"error\"}";
        std::fs::write(attempt.join("receipt.json"), receipt).unwrap();
        let row = failure("Individual quota reached. Resets in 30m.");
        std::fs::write(
            attempt.join("transcripts/game-seat1.jsonl"),
            format!("{row}\n{{broken partial"),
        )
        .unwrap();
        let now = crate::schedule::now_unix();
        let evidence = inspect_attempt(&attempt, now).unwrap();
        assert_eq!(evidence.len(), 1);
        let reset = evidence[0]["reset_not_before_unix"].as_f64().unwrap();
        assert!(reset > now && reset <= now + 1801.0);
        assert!(inspect_attempt(&attempt, reset + 1.0).unwrap().is_empty());
        assert!(check_startup(temp.path(), now).is_err());
        assert!(check_startup(temp.path(), reset + 1.0).is_err());
        assert_eq!(
            std::fs::read(attempt.join("receipt.json")).unwrap(),
            receipt
        );
        let hold = std::fs::read_to_string(temp.path().join(HOLD_FILE)).unwrap();
        assert!(!hold.contains("Individual quota reached"));
        assert!(!hold.contains("broken partial"));
    }
    #[test]
    fn malformed_reset_is_review_only_and_malformed_hold_is_fail_closed() {
        let temp = crate::audit::tests::TestDir::new("quota-hold-invalid");
        let attempt = temp.path().join("attempts/attempt-000000001");
        std::fs::create_dir_all(attempt.join("transcripts")).unwrap();
        let row = failure("Individual quota reached. Resets in soon.");
        std::fs::write(
            attempt.join("transcripts/game-seat0.jsonl"),
            format!("{row}\n"),
        )
        .unwrap();
        let evidence = inspect_attempt(&attempt, 0.0).unwrap();
        assert!(evidence[0]["reset_not_before_unix"].is_null());
        assert_eq!(evidence[0]["requires_reset_review"], true);
        std::fs::write(temp.path().join(HOLD_FILE), b"broken").unwrap();
        assert!(check_startup(temp.path(), 0.0).is_err());
    }
}
