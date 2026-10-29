use super::{privacy, strict_json};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
const UNIT: &[&str] = &[
    "MulliganKeep",
    "MulliganTake",
    "PassPriority",
    "Concede",
    "CancelCast",
    "OfferDraw",
];
const AUTOMATION: &[&str] = &["SetPriorityHold", "SetAbilityYield", "SetAbilityPolicy"];
const VALUE: &[&str] = &[
    "ChooseManaAbility",
    "ChooseDamageSource",
    "ChooseDamageEffect",
    "AllocatePrevention",
    "TakeGrantedAction",
    "PlayLand",
    "CastSpell",
    "ActivateManaAbility",
    "ActivateAbility",
    "DeclareAttackers",
    "DeclareBlockers",
    "ChooseObjects",
    "ChooseTargets",
    "ChooseTargetBatch",
    "Suspend",
    "Arrange",
    "ChooseColor",
    "ChooseSubtype",
    "ChooseCardName",
    "ChooseMode",
    "ChooseNumber",
    "ChoosePlayer",
    "YesNo",
    "PayWindow",
];
fn fields(kind: &str) -> Option<&'static [&'static str]> {
    match kind {
        "header" => Some(&["kind", "record", "build", "preset", "hash", "seen"]),
        "input" => Some(&["kind", "n", "at", "seat", "by", "action", "hash", "seen"]),
        "chair" => Some(&["kind", "n", "at", "seat", "change"]),
        "declared_mind" => Some(&["kind", "n", "at", "seat", "mind"]),
        "end" => Some(&["kind", "n", "at", "winners", "reason", "turn"]),
        _ => None,
    }
}
pub(super) fn hex(value: &str, width: usize) -> bool {
    value.len() == width
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Validate anonymized complete duel records, preserving provenance without labels.
///
/// # Errors
/// Rejects malformed, incomplete, identifiable or unsupported records.
pub fn validate(raw: &[u8], neural_seat: u64) -> Result<Value> {
    ensure!(neural_seat < 2, "invalid neural seat");
    let lines = std::str::from_utf8(raw)?
        .lines()
        .filter(|s| !s.trim().is_empty())
        .map(|s| strict_json::parse(s.as_bytes()))
        .collect::<Result<Vec<_>>>()?;
    ensure!(lines.len() >= 3, "record is empty or incomplete");
    ensure!(
        lines.iter().all(Value::is_object),
        "record line is not an object"
    );
    ensure!(
        lines[0]["kind"] == "header" && lines[lines.len() - 1]["kind"] == "end",
        "only complete Header-to-End records are accepted"
    );
    ensure!(
        lines[0]["record"].as_u64() == Some(1),
        "unsupported record version"
    );
    for row in &lines {
        privacy::check(row, "")?;
    }
    let seats = lines[0]["preset"]["seats"]
        .as_array()
        .context("invalid preset")?;
    ensure!(
        seats.len() == 2
            && seats
                .iter()
                .all(|s| s.as_object().is_some_and(|s| s.contains_key("controller"))),
        "exactly two seats with controllers required"
    );
    let mut counts = [BTreeMap::new(), BTreeMap::new()];
    let mut actions = [BTreeMap::new(), BTreeMap::new()];
    let mut declared = BTreeSet::new();
    let mut input_roles = BTreeSet::new();
    let mut current = None;
    for (index, row) in lines.iter().enumerate() {
        let kind = row["kind"].as_str().context("unknown record kind")?;
        let seat = check_row(row, kind, index, lines.len())?;
        if seat == Some(1 - neural_seat) {
            if kind == "declared_mind" {
                let mind = row["mind"]["kind"]
                    .as_str()
                    .context("invalid declared mind")?
                    .to_owned();
                current = Some(mind.clone());
                declared.insert(Some(mind));
            } else if kind == "input" {
                input_roles.insert(current.clone());
            }
        }
        if kind == "input" {
            let origin = row["by"]
                .as_str()
                .filter(|s| ["seat", "clock", "house", "stand_in"].contains(s))
                .context("unknown input origin")?;
            let action_kind = action_kind(&row["action"])?;
            let seat = seat.context("incomplete input")? as usize;
            *counts[seat].entry(origin.to_owned()).or_insert(0u64) += 1;
            *actions[seat]
                .entry(if AUTOMATION.contains(&action_kind) {
                    "automation"
                } else {
                    "decision"
                })
                .or_insert(0u64) += 1;
        }
    }
    declared.extend(input_roles);
    let role = if declared == BTreeSet::from([Some("human".to_owned())]) {
        "confirmed_human"
    } else if declared == BTreeSet::from([Some("house".to_owned())]) {
        "excluded_house"
    } else {
        "quarantine_unknown_or_mixed"
    };
    let end = &lines[lines.len() - 1];
    let winners = end["winners"]
        .as_array()
        .context("invalid completed outcome")?;
    ensure!(
        winners.iter().all(|s| s.as_u64().is_some_and(|s| s < 2)),
        "invalid completed outcome"
    );
    ensure!(end["reason"].is_string(), "missing completion reason");
    Ok(
        json!({"neural_seat":neural_seat,"human_seat":1-neural_seat,"human_role_status":role,"behavior_logprob_available":false,"inputs":counts.iter().flat_map(BTreeMap::values).sum::<u64>(),"origins_by_seat":counts,"actions_by_seat":actions,"winners":winners,"terminal_turn":end["turn"],"replay_verified":false}),
    )
}

fn action_kind(action: &Value) -> Result<&str> {
    if let Some(name) = action.as_str() {
        ensure!(UNIT.contains(&name), "unknown or malformed action kind");
        Ok(name)
    } else {
        let object = action
            .as_object()
            .filter(|o| o.len() == 1)
            .context("invalid action representation")?;
        let name = object.keys().next().context("missing action")?.as_str();
        ensure!(
            VALUE.contains(&name) || AUTOMATION.contains(&name),
            "unknown or malformed action kind"
        );
        Ok(name)
    }
}

fn check_row(row: &Value, kind: &str, index: usize, length: usize) -> Result<Option<u64>> {
    let allowed = fields(kind).context("unknown record fields/kind")?;
    ensure!(
        row.as_object()
            .context("record line is not an object")?
            .keys()
            .all(|k| allowed.contains(&k.as_str())),
        "unknown record fields/kind"
    );
    ensure!(
        allowed
            .iter()
            .filter(|k| !["seen", "turn"].contains(k))
            .all(|k| row.get(k).is_some()),
        "missing record fields"
    );
    ensure!(
        (kind == "header") == (index == 0),
        "duplicate or misplaced Header"
    );
    ensure!(
        (kind == "end") == (index == length - 1),
        "duplicate or misplaced End"
    );
    if kind != "header" {
        ensure!(row["at"].as_u64() == Some(0), "host clock not anonymized");
        ensure!(
            row["n"].as_u64() == Some(index as u64 - 1),
            "broken record sequence"
        );
    }
    if ["header", "input"].contains(&kind) {
        ensure!(
            row["hash"].as_str().is_some_and(|s| hex(s, 16)),
            "missing snapshot digest"
        );
    }
    row.get("seat")
        .map(|s| s.as_u64().filter(|s| *s < 2).context("invalid seat"))
        .transpose()
}
