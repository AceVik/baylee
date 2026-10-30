//! Recompute conservative action alignments instead of trusting a claimed LLM origin.
use anyhow::{Context, ensure};
use serde_json::{Value, json};

struct Send {
    action: Value,
    source: &'static str,
    question: Option<u64>,
}

pub(super) fn verify(
    transcript: &[Value],
    inputs: &[Value],
    claimed: &Value,
) -> anyhow::Result<Vec<bool>> {
    ensure!(
        transcript.iter().any(|v| v["event"] == "over"),
        "LLM transcript lacks GameOver"
    );
    let mut sends: Vec<Send> = Vec::new();
    for row in transcript {
        match row["event"].as_str() {
            Some("answered" | "standing") => {
                let source = if row["event"] == "standing" {
                    "standing"
                } else {
                    match row["by"].as_str() {
                        Some("mind") => "llm",
                        Some("house") => "house",
                        Some("standing") => "standing",
                        _ => "other",
                    }
                };
                ensure!(source != "other", "Unresolved transcript answer source");
                sends.push(Send {
                    action: row["action"].clone(),
                    source,
                    question: row["question"].as_u64(),
                });
            }
            Some("refused") => {
                if let Some(index) = sends.iter().rposition(|s| {
                    s.question == row["question"].as_u64() && s.action == row["action"]
                }) {
                    sends.remove(index);
                }
            }
            _ => {}
        }
    }
    let mut earliest = Vec::new();
    let mut cursor = 0;
    for input in inputs {
        while cursor < sends.len() && sends[cursor].action != input["action"] {
            cursor += 1;
        }
        ensure!(
            cursor < sends.len(),
            "Executed action lacks ordered transcript prefix"
        );
        earliest.push(cursor);
        cursor += 1;
    }
    let mut latest = vec![0; inputs.len()];
    let mut cursor = sends.len();
    for (index, input) in inputs.iter().enumerate().rev() {
        while cursor > 0 && sends[cursor - 1].action != input["action"] {
            cursor -= 1;
        }
        ensure!(
            cursor > 0,
            "Executed action lacks ordered transcript suffix"
        );
        latest[index] = cursor - 1;
        cursor -= 1;
    }
    let claims = claimed["inputs"]
        .as_array()
        .context("LLM ordered origins missing")?;
    ensure!(
        claims.len() == inputs.len() && claimed["ppo_eligible"] == false,
        "LLM origin count/PPO mismatch"
    );
    let mut labels = Vec::new();
    for (i, input) in inputs.iter().enumerate() {
        let low = if i == 0 { 0 } else { earliest[i - 1] + 1 };
        let high = if i + 1 == inputs.len() {
            sends.len()
        } else {
            latest[i + 1]
        };
        let matching: Vec<_> = sends[low..high]
            .iter()
            .filter(|s| s.action == input["action"])
            .collect();
        let origins: Vec<_> = ["llm", "house", "standing", "other"]
            .into_iter()
            .filter(|s| matching.iter().any(|v| v.source == *s))
            .collect();
        let claim = &claims[i];
        ensure!(
            claim["record_input_n"] == input["n"]
                && claim["action"] == input["action"]
                && claim["possible_sources"] == json!(origins)
                && claim["ambiguous"] == json!(origins.len() > 1)
                && claim["possible_send_count"] == json!(matching.len())
                && claim["policy_target_admitted"] == false,
            "LLM executed-action/origin claim mismatch"
        );
        labels.push(origins == ["llm"]);
    }
    Ok(labels)
}
