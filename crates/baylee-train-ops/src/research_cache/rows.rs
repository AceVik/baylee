use super::*;
use crate::research_contract::{FRAME_CONTRACT, ResearchReturn, contract};
use serde_json::json;
use std::collections::BTreeMap;

pub(super) struct Metadata {
    pub episode: u32,
    pub seat: u32,
    pub cursor: u64,
    pub flags: u32,
    pub genuine: bool,
}

pub(super) fn episode_return(episode: &Value, learner: u64) -> Result<Value> {
    ensure!(
        episode["kind"] == "research_termination" && episode["target_contract"] == contract(),
        "source episode target contract changed"
    );
    let mut result = serde_json::Map::new();
    for name in [
        "termination_kind",
        "training_utility",
        "mtg_outcome",
        "bootstrap",
    ] {
        result.insert(
            name.into(),
            episode
                .get(name)
                .context("source episode missing return field")?
                .clone(),
        );
    }
    let value = Value::Object(result);
    let ret: ResearchReturn = serde_json::from_value(value.clone())?;
    ensure!(!ret.bootstrap, "research v1 cannot bootstrap");
    let expected = match ret.termination_kind.as_str() {
        "unknown" => {
            ensure!(ret.mtg_outcome.is_none(), "unknown MTG outcome changed");
            None
        }
        "learner_announcement_loop" => {
            ensure!(
                ret.mtg_outcome.is_none(),
                "learner loop fabricated an MTG outcome"
            );
            Some(0.0)
        }
        "mtg_terminal" => {
            let outcome = ret
                .mtg_outcome
                .as_ref()
                .context("natural return lacks MTG outcome")?;
            Some(if outcome == &json!({"kind":"draw"}) {
                0.5
            } else {
                let seat = uint(&outcome["seat"])?;
                ensure!(
                    seat < 2 && *outcome == json!({"kind":"won","seat":seat}),
                    "unsupported 1v1 natural result"
                );
                if seat == learner { 1.0 } else { 0.0 }
            })
        }
        _ => anyhow::bail!("unsupported research termination"),
    };
    ensure!(
        ret.training_utility == expected,
        "research utility mapping changed"
    );
    Ok(value)
}

fn array<'a>(value: &'a Value, name: &str) -> Result<&'a Vec<Value>> {
    value[name]
        .as_array()
        .with_context(|| format!("missing research array {name}"))
}
fn matrix(value: &Value, name: &str, width: usize) -> Result<usize> {
    let rows = array(value, name)?;
    ensure!(
        rows.iter()
            .all(|row| row.as_array().is_some_and(|row| row.len() == width)),
        "research matrix width mismatch: {name}"
    );
    Ok(rows.len())
}
fn finite(value: &Value) -> Result<()> {
    match value {
        Value::Number(n) => ensure!(
            n.as_f64().is_some_and(f64::is_finite),
            "nonfinite research numeric field"
        ),
        Value::Array(values) => {
            for value in values {
                finite(value)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                finite(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn references(rows: &[Value], roots: &BTreeMap<String, u32>) -> Result<()> {
    for row in rows {
        for reference in row.as_array().context("invalid public rule references")? {
            ensure!(
                roots.contains_key(reference["key"].as_str().context("public rule key")?)
                    && matches!(reference["role"].as_u64(), Some(1..=7)),
                "unrepresented public rule reference"
            );
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "One strict research frame schema without legacy defaults"
)]
pub(super) fn validate(
    raw: &[u8],
    history: &source::History,
    encoder: u64,
    cursor: u64,
    roots: &BTreeMap<String, u32>,
) -> Result<Metadata> {
    let row: Value = serde_json::from_slice(raw)?;
    ensure!(
        row.as_object().is_some_and(|object| object.len() == 5)
            && row["schema"] == FRAME_CONTRACT
            && row["returns"] == history.receipt["returns"],
        "research frame/return contract mismatch"
    );
    finite(&row)?;
    let f = row.get("features").context("research features")?;
    for name in ["outcome", "aux", "is_terminal", "shadow_label", "llm_label"] {
        ensure!(
            f.get(name).is_none(),
            "legacy/teacher labels in research frame"
        );
    }
    let episode = u32::try_from(uint(&f["game_id"])?)?;
    let seat = u32::try_from(uint(&f["seat"])?)?;
    ensure!(
        u64::from(episode) == uint(&history.receipt["episode_id"])?
            && u64::from(seat) == history.learner
            && uint(&f["step_cursor"])? == cursor,
        "research history key/cursor mismatch"
    );
    let terminal = row["training_terminal"]
        .as_bool()
        .context("research terminal flag")?;
    ensure!(
        terminal == (cursor + 1 == uint(&history.receipt["frames"])?),
        "research history terminal is not its unique final frame"
    );
    let decision = f["is_decision"]
        .as_bool()
        .context("research decision flag")?;
    let state = &f["state"];
    let entities = matrix(state, "entity_feat", 128)?;
    let candidates = matrix(f, "cand_feat", 48)?;
    ensure!(
        entities <= 1024
            && candidates <= 65535
            && array(f, "pairs")?.len() <= 65535
            && array(state, "edges")?.len() <= 8192
            && matrix(state, "seat_feat", 64)? == 8
            && array(state, "global_feat")?.len() == if encoder == 9 { 82 } else { 80 },
        "research feature dimensions/caps mismatch"
    );
    for name in [
        "entity_object_ids",
        "entity_group_members",
        "entity_card_ids",
        "entity_face_ids",
        "entity_token_ids",
        "entity_rule_refs",
        "entity_zone_ids",
        "entity_rel_controller",
    ] {
        ensure!(
            array(state, name)?.len() == entities,
            "research entity column mismatch: {name}"
        );
    }
    for name in [
        "cand_entity_idx",
        "cand_rel_seat_idx",
        "cand_card_id",
        "cand_face_id",
        "cand_ability_idx",
        "cand_rule_refs",
    ] {
        ensure!(
            array(f, name)?.len() == candidates,
            "research candidate column mismatch: {name}"
        );
    }
    references(array(state, "entity_rule_refs")?, roots)?;
    references(array(f, "cand_rule_refs")?, roots)?;
    let p = &f["provenance"];
    let has_lp = p["has_logprob"].as_bool().context("research LP flag")?;
    let lp = p["behavior_logprob"]
        .as_f64()
        .context("research behavior LP")?;
    let temperature = p["temperature"].as_f64().context("research temperature")?;
    let net = p["source"] == "Net";
    ensure!(
        net || p["source"] == "Forced",
        "unsupported research policy source"
    );
    ensure!(
        decision || (!net && !has_lp && lp == 0.0),
        "passive history acquired actor provenance"
    );
    if net {
        ensure!(
            decision && p["thinking_steps"] == 2 && (temperature - 0.7).abs() < 1e-6 && lp <= 0.0,
            "research neural behavior contract changed"
        );
    } else {
        ensure!(
            !has_lp && lp == 0.0,
            "forced frame claims actor probability"
        );
    }
    let head = uint(&f["head"])?;
    let t = &f["target"];
    let size = if head == 2 {
        array(f, "pairs")?.len()
    } else {
        candidates
    };
    let mut free = false;
    if decision && matches!(head, 1 | 2) {
        for name in ["multi_mask", "free_mask"] {
            let values = array(t, name)?;
            ensure!(
                values.len() == size && values.iter().all(|v| matches!(v.as_u64(), Some(0 | 1))),
                "invalid original research action mask"
            );
            free |= name == "free_mask" && values.iter().any(|v| *v == 1);
        }
    }
    for pick in array(t, "pick_sequence")? {
        ensure!(
            usize::try_from(uint(pick)?)? < size,
            "research pick sequence outside menu"
        );
    }
    let choice = match head {
        0 => {
            if decision {
                ensure!(
                    usize::try_from(uint(&t["single_idx"])?)? < candidates,
                    "research categorical target outside menu"
                );
            }
            candidates > 1
        }
        1 | 2 => free && temperature > 0.35,
        3 => {
            ensure!(
                !has_lp && lp == 0.0,
                "scalar frame claims categorical actor LP"
            );
            false
        }
        _ => anyhow::bail!("unknown research policy head"),
    };
    if net {
        ensure!(
            has_lp != (head == 3),
            "categorical/scalar LP presence changed"
        );
    }
    let candidate = decision && net && has_lp;
    ensure!(
        row["actor_eligible"] == candidate,
        "original actor-candidate flag changed"
    );
    Ok(Metadata {
        episode,
        seat,
        cursor,
        flags: u32::from(decision) | (u32::from(terminal) << 1) | (u32::from(candidate) << 2),
        genuine: candidate && choice,
    })
}
