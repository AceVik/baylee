//! Existing BST4 little-endian frame ABI, shared by native cache and admission.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeMap;
/// Fixed header length for BST4 version 1.
pub const HEADER_SIZE: usize = 136;
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a [Value]> {
    v[key]
        .as_array()
        .map(Vec::as_slice)
        .with_context(|| format!("missing array {key}"))
}
fn uint(v: &Value, default: u64) -> Result<u32> {
    Ok(u32::try_from(if v.is_null() {
        default
    } else {
        v.as_u64().context("invalid unsigned frame field")?
    })?)
}
fn int(v: &Value, default: i64) -> Result<i32> {
    Ok(i32::try_from(if v.is_null() {
        default
    } else {
        v.as_i64().context("invalid signed frame field")?
    })?)
}
#[allow(clippy::cast_possible_truncation)]
fn float(v: &Value, default: f64) -> Result<f32> {
    let n = if v.is_null() {
        default
    } else {
        v.as_f64().context("invalid frame float")?
    };
    ensure!(n.is_finite(), "nonfinite frame float");
    let n = n as f32;
    ensure!(n.is_finite(), "frame float exceeds f32");
    Ok(n)
}
fn width(rows: &[Value]) -> Result<usize> {
    let width = rows
        .first()
        .map_or(Some(0), |r| r.as_array().map(Vec::len))
        .context("invalid frame matrix")?;
    ensure!(
        rows.iter()
            .all(|r| r.as_array().is_some_and(|r| r.len() == width)),
        "ragged frame matrix"
    );
    Ok(width)
}
fn floats(out: &mut Vec<u8>, values: &[Value]) -> Result<()> {
    for v in values {
        out.extend_from_slice(&float(v, 0.0)?.to_le_bytes());
    }
    Ok(())
}
fn matrix(out: &mut Vec<u8>, rows: &[Value]) -> Result<()> {
    for row in rows {
        floats(out, row.as_array().context("invalid frame matrix")?)?;
    }
    Ok(())
}
fn refs(
    out: &mut Vec<u8>,
    rows: &[Value],
    columns: usize,
    roots: &BTreeMap<String, u32>,
) -> Result<()> {
    for row in rows {
        let row = row.as_array().context("invalid rule references")?;
        for column in 0..columns {
            if let Some(reference) = row.get(column) {
                let key = reference["key"].as_str().context("missing rule key")?;
                let root = roots
                    .get(key)
                    .context("Unrepresented public rule reference")?;
                let role = uint(&reference["role"], 0)?;
                ensure!((1..=7).contains(&role), "Unrepresented public rule role");
                out.extend_from_slice(&root.to_le_bytes());
                out.extend_from_slice(&role.to_le_bytes());
            } else {
                out.extend_from_slice(&[0; 8]);
            }
        }
    }
    Ok(())
}
fn rule_columns(rows: &[Value]) -> Result<usize> {
    Ok(rows
        .iter()
        .map(|r| {
            r.as_array()
                .map(Vec::len)
                .context("invalid rule references")
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .max()
        .unwrap_or(0)
        .max(1))
}
/// Pack a legacy JSON trajectory frame into the unchanged BST4 tensor ABI.
///
/// # Errors
/// Rejects invalid shapes, absent rules, unsupported roles or out-of-range numbers.
#[allow(clippy::too_many_lines)] // One contiguous versioned wire layout.
pub fn pack_step(step: &Value, roots: &BTreeMap<String, u32>) -> Result<Vec<u8>> {
    let state = &step["state"];
    let entities = array(state, "entity_feat")?;
    let cards = array(state, "entity_card_ids")?;
    let er = array(state, "entity_rule_refs")?;
    let candidates = array(step, "cand_feat")?;
    let cr = array(step, "cand_rule_refs")?;
    ensure!(
        entities.len() == cards.len() && entities.len() == er.len() && candidates.len() == cr.len(),
        "Missing entity/action rule references"
    );
    let ew = width(entities)?;
    let cw = width(candidates)?;
    let ec = rule_columns(er)?;
    let cc = rule_columns(cr)?;
    let seats = array(state, "seat_feat")?;
    let sw = width(seats)?;
    let globals = array(state, "global_feat")?;
    let edges = array(state, "edges")?;
    let pairs = array(step, "pairs")?;
    let target = &step["target"];
    let empty = Vec::new();
    let multi = target["multi_mask"].as_array().unwrap_or(&empty);
    let free = target["free_mask"].as_array().unwrap_or(&empty);
    let decision = step["is_decision"].as_bool().unwrap_or(true);
    let terminal = step["is_terminal"].as_bool().unwrap_or(false);
    let provenance = &step["provenance"];
    let flags = u32::from(decision)
        | (u32::from(terminal) << 1)
        | (u32::from(provenance["has_logprob"].as_bool().unwrap_or(false)) << 2);
    let source = match provenance["source"].as_str() {
        Some("Net") => 0,
        Some("Forced") => 2,
        Some("Search") => 3,
        Some("External") => 4,
        _ => provenance["source"].as_u64().unwrap_or(1),
    };
    let mut out = b"BST4".to_vec();
    for value in [
        1,
        uint(&step["game_id"], 0)?,
        uint(&step["seat"], 0)?,
        uint(&step["step_cursor"], 0)?,
        uint(&step["turn"], 0)?,
        flags,
        uint(&step["head"], 0)?,
        uint(&step["pending_kind"], 0)?,
        u32::try_from(source)?,
        0,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for value in [
        int(&provenance["thinking_steps"], 0)?,
        int(&target["single_idx"], 0)?,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    let aux = &step["aux"];
    for value in [
        float(&step["loss_weight"], if decision { 1.0 } else { 0.0 })?,
        float(&target["scalar_norm"], 0.0)?,
        float(&step["outcome"], 0.0)?,
        float(&aux["turns_remaining_norm"], 0.0)?,
        float(&aux["my_final_life_norm"], 0.0)?,
        float(&aux["opp_min_final_life_norm"], 0.0)?,
        float(&aux["next_life_delta_norm"], 0.0)?,
        float(&provenance["behavior_logprob"], 0.0)?,
        float(&provenance["temperature"], 0.0)?,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for dim in [
        entities.len(),
        ew,
        ec,
        sw,
        globals.len(),
        edges.len(),
        candidates.len(),
        cw,
        cc,
        pairs.len(),
        multi.len(),
        free.len(),
    ] {
        out.extend_from_slice(&u32::try_from(dim)?.to_le_bytes());
    }
    matrix(&mut out, entities)?;
    for card in cards {
        out.extend_from_slice(&uint(card, 0)?.to_le_bytes());
    }
    for i in 0..entities.len() {
        out.push(u8::try_from(uint(&state["entity_face_ids"][i], 0)?)?);
    }
    for i in 0..entities.len() {
        out.extend_from_slice(
            &u16::try_from(uint(&state["entity_token_ids"][i], 0)?)?.to_le_bytes(),
        );
    }
    refs(&mut out, er, ec, roots)?;
    matrix(&mut out, seats)?;
    floats(&mut out, globals)?;
    for edge in edges {
        let edge = edge
            .as_array()
            .filter(|e| e.len() == 3)
            .context("invalid edge")?;
        for v in edge {
            out.extend_from_slice(&int(v, 0)?.to_le_bytes());
        }
    }
    matrix(&mut out, candidates)?;
    for i in 0..candidates.len() {
        out.extend_from_slice(&i16::try_from(int(&step["cand_entity_idx"][i], -1)?)?.to_le_bytes());
    }
    for i in 0..candidates.len() {
        out.extend_from_slice(
            &i8::try_from(int(&step["cand_rel_seat_idx"][i], -1)?)?.to_le_bytes(),
        );
    }
    let card_ids = step["cand_card_id"]
        .as_array()
        .filter(|r| r.len() >= candidates.len());
    let face_ids = step["cand_face_id"]
        .as_array()
        .filter(|r| r.len() >= candidates.len());
    for i in 0..candidates.len() {
        out.extend_from_slice(
            &uint(
                card_ids.map_or(&Value::Null, |r| &r[i]),
                u64::from(u32::MAX),
            )?
            .to_le_bytes(),
        );
    }
    for i in 0..candidates.len() {
        out.push(u8::try_from(uint(
            face_ids.map_or(&Value::Null, |r| &r[i]),
            255,
        )?)?);
    }
    refs(&mut out, cr, cc, roots)?;
    for pair in pairs {
        let pair = pair
            .as_array()
            .filter(|p| p.len() == 2)
            .context("invalid pair")?;
        for v in pair {
            out.extend_from_slice(&i16::try_from(int(v, 0)?)?.to_le_bytes());
        }
    }
    for mask in [multi, free] {
        for v in mask {
            out.push(u8::try_from(uint(v, 0)?)?);
        }
    }
    Ok(out)
}

fn word(raw: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        raw.get(offset..offset + 4)
            .context("truncated BST4 frame")?
            .try_into()?,
    ))
}
fn real(raw: &[u8], offset: usize) -> Result<f64> {
    let value = f32::from_bits(word(raw, offset)?);
    ensure!(value.is_finite(), "nonfinite BST4 header");
    Ok(f64::from(value))
}
/// Decode admission metadata and conditional masks from a BST4 frame.
///
/// # Errors
/// Rejects truncated or unknown frames and inconsistent payload sizes.
pub fn metadata(raw: &[u8]) -> Result<Value> {
    ensure!(
        raw.starts_with(b"BST4") && word(raw, 4)? == 1 && raw.len() >= HEADER_SIZE,
        "invalid BST4 frame header"
    );
    let dims = (0..12)
        .map(|i| word(raw, 88 + i * 4).map(u64::from))
        .collect::<Result<Vec<_>>>()?;
    let [
        entities,
        ew,
        ec,
        sw,
        gw,
        edges,
        candidates,
        cw,
        cc,
        pairs,
        multi,
        free,
    ]: [u64; 12] = dims
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid BST4 dimensions"))?;
    let terms = [
        entities.checked_mul(ew).and_then(|n| n.checked_mul(4)),
        entities.checked_mul(7),
        entities.checked_mul(ec).and_then(|n| n.checked_mul(8)),
        sw.checked_mul(32),
        gw.checked_mul(4),
        edges.checked_mul(12),
        candidates.checked_mul(cw).and_then(|n| n.checked_mul(4)),
        candidates.checked_mul(8),
        candidates.checked_mul(cc).and_then(|n| n.checked_mul(8)),
        pairs.checked_mul(4),
        Some(multi),
        Some(free),
    ];
    let length = terms
        .into_iter()
        .try_fold(HEADER_SIZE as u64, |sum, n| sum.checked_add(n?).or(None))
        .context("BST4 dimensions overflow")?;
    ensure!(
        length == raw.len() as u64,
        "BST4 payload shape/length mismatch"
    );
    let multi = usize::try_from(multi)?;
    let free = usize::try_from(free)?;
    let start = raw.len() - multi - free;
    let flags = word(raw, 24)?;
    let source = match word(raw, 36)? {
        0 => "Net",
        1 => "House",
        2 => "Forced",
        3 => "Search",
        4 => "External",
        _ => anyhow::bail!("unknown BST4 provenance"),
    };
    Ok(
        json!({"global_feat_width":gw,"game_id":word(raw,8)?,"seat":word(raw,12)?,"step_cursor":word(raw,16)?,"turn":word(raw,20)?,"is_decision":flags&1!=0,"is_terminal":flags&2!=0,"head":word(raw,28)?,"pending_kind":word(raw,32)?,"loss_weight":real(raw,52)?,"outcome":real(raw,60)?,"cand_feat":vec![Value::Null;usize::try_from(candidates)?],"pairs":vec![Value::Null;usize::try_from(pairs)?],"target":{"single_idx":i32::from_le_bytes(raw[48..52].try_into()?),"scalar_norm":real(raw,56)?,"multi_mask":raw[start..start+multi],"free_mask":raw[start+multi..]},"provenance":{"source":source,"thinking_steps":i32::from_le_bytes(raw[44..48].try_into()?),"has_logprob":flags&4!=0,"behavior_logprob":real(raw,80)?,"temperature":real(raw,84)?}}),
    )
}
