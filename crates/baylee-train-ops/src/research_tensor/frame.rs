use super::{arrays::Tensor, catalog::Catalog, *};
use std::collections::BTreeMap;

fn integer(v: &Value) -> Result<i64> {
    v.as_i64().context("missing signed tensor field")
}
#[allow(clippy::cast_possible_truncation)]
fn real(v: &Value) -> Result<f32> {
    let n = v.as_f64().context("missing float tensor field")?;
    let f = n as f32;
    ensure!(n.is_finite() && f.is_finite(), "float exceeds FP32");
    Ok(f)
}
fn boolean(v: &Value) -> Result<bool> {
    v.as_bool().context("missing boolean tensor field")
}
fn matrix(v: &Value, name: &str, width: usize) -> Result<Tensor> {
    let rows = array(v, name)?;
    let mut values = Vec::new();
    for row in rows {
        let row = row.as_array().context("tensor matrix row")?;
        ensure!(row.len() == width, "tensor matrix width");
        for n in row {
            values.push(real(n)?);
        }
    }
    let count = rows.len().max(1);
    values.resize(count * width, 0.);
    Ok(Tensor::floats(vec![count, width], values))
}
fn rules(rows: &[Value], catalog: &Catalog) -> Result<Tensor> {
    let cols = rows
        .iter()
        .map(|v| v.as_array().map(Vec::len).context("research rule row"))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .max()
        .unwrap_or(0)
        .max(1);
    let elements = rows
        .len()
        .max(1)
        .checked_mul(cols)
        .and_then(|n| n.checked_mul(2))
        .context("semantic reference tensor overflow")?;
    ensure!(
        elements <= MAX_BYTES / 8,
        "semantic reference tensor exceeds memory bound"
    );
    let mut out = vec![0; elements];
    for (r, row) in rows.iter().enumerate() {
        for (c, v) in row
            .as_array()
            .context("rule references")?
            .iter()
            .enumerate()
        {
            let root = *catalog
                .roots
                .get(v["key"].as_str().context("rule key")?)
                .context("unrepresented rule")?;
            let role = integer(&v["role"])?;
            ensure!((1..=7).contains(&role), "invalid rule role");
            out[(r * cols + c) * 2] = i64::from(root);
            out[(r * cols + c) * 2 + 1] = role;
        }
    }
    Ok(Tensor::ints(vec![rows.len().max(1), cols, 2], out))
}
fn mask(values: &[Value], size: usize) -> Result<Vec<f32>> {
    ensure!(values.len() == size, "original tensor mask length changed");
    values
        .iter()
        .map(|v| {
            let n = integer(v)?;
            ensure!(matches!(n, 0 | 1), "nonbinary original mask");
            Ok(if n == 0 { 0. } else { 1. })
        })
        .collect()
}

#[allow(
    clippy::too_many_lines,
    reason = "Strict named model inputs and research-only targets, no legacy defaults"
)]
pub(super) fn pack(row: &Value, catalog: &Catalog) -> Result<BTreeMap<String, Tensor>> {
    let features = &row["features"];
    let state = &features["state"];
    let target = &features["target"];
    let provenance = &features["provenance"];
    let n = array(state, "entity_feat")?.len();
    let c = array(features, "cand_feat")?.len();
    let pairs = array(features, "pairs")?;
    let mut out = BTreeMap::new();
    out.insert("entity_feat".into(), matrix(state, "entity_feat", 128)?);
    out.insert("cand_feat".into(), matrix(features, "cand_feat", 48)?);
    out.insert("seat_feat".into(), matrix(state, "seat_feat", 64)?);
    let global = array(state, "global_feat")?;
    out.insert(
        "global_feat".into(),
        Tensor::floats(
            vec![global.len()],
            global.iter().map(real).collect::<Result<Vec<_>>>()?,
        ),
    );
    for (name, count) in [
        ("entity_mask", n),
        ("cand_mask", c),
        ("pair_mask", pairs.len()),
    ] {
        out.insert(
            name.into(),
            Tensor::bools(vec![count.max(1)], (0..count.max(1)).map(|i| i < count)),
        );
    }
    let mut dsl = Vec::new();
    for i in 0..n {
        dsl.extend(
            catalog.entity(
                state["entity_card_ids"][i]
                    .as_u64()
                    .context("entity card")?,
                state["entity_face_ids"][i]
                    .as_u64()
                    .context("entity face")?,
                state["entity_token_ids"][i]
                    .as_u64()
                    .context("entity token")?,
            )?,
        );
    }
    let entity_dsl = dsl.clone();
    dsl.resize(n.max(1) * 160, 0.);
    out.insert(
        "entity_dsl".into(),
        Tensor::floats(vec![n.max(1), 160], dsl),
    );
    let mut cdsl = Vec::new();
    let mut entities = Vec::new();
    let mut seats = Vec::new();
    for i in 0..c {
        let e = integer(&features["cand_entity_idx"][i])?;
        let seat = integer(&features["cand_rel_seat_idx"][i])?;
        ensure!(
            e >= -1 && (e < 0 || usize::try_from(e)? < n) && (-1..8).contains(&seat),
            "unrepresented candidate reference"
        );
        entities.push(e);
        seats.push(seat);
        let card = features["cand_card_id"][i]
            .as_u64()
            .context("candidate card")?;
        let face = features["cand_face_id"][i]
            .as_u64()
            .context("candidate face")?;
        let features = if face == 255 && e >= 0 {
            let e = usize::try_from(e)?;
            entity_dsl[e * 160..(e + 1) * 160].to_vec()
        } else if card == u64::from(u32::MAX) {
            ensure!(face == 255, "named face without card");
            vec![0.; 160]
        } else {
            catalog.card(card, face)?
        };
        cdsl.extend(features);
    }
    cdsl.resize(c.max(1) * 160, 0.);
    entities.resize(c.max(1), -1);
    seats.resize(c.max(1), -1);
    out.insert("cand_dsl".into(), Tensor::floats(vec![c.max(1), 160], cdsl));
    out.insert(
        "cand_entity_idx".into(),
        Tensor::ints(vec![c.max(1)], entities),
    );
    out.insert("cand_seat_idx".into(), Tensor::ints(vec![c.max(1)], seats));
    out.insert(
        "entity_rules".into(),
        rules(array(state, "entity_rule_refs")?, catalog)?,
    );
    out.insert(
        "cand_rules".into(),
        rules(array(features, "cand_rule_refs")?, catalog)?,
    );
    let edges = array(state, "edges")?;
    let mut edge_idx = Vec::new();
    let mut edge_types = Vec::new();
    for edge in edges {
        let edge = edge.as_array().context("edge row")?;
        ensure!(edge.len() == 3, "edge width");
        for v in &edge[..2] {
            let k = integer(v)?;
            ensure!(
                k >= 0 && usize::try_from(k)? < 9 + n,
                "unrepresented edge endpoint"
            );
            edge_idx.push(k);
        }
        let kind = integer(&edge[2])?;
        ensure!((0..14).contains(&kind), "edge type");
        edge_types.push(kind);
    }
    edge_idx.resize(edges.len().max(1) * 2, 0);
    edge_types.resize(edges.len().max(1), 0);
    out.insert(
        "edge_index".into(),
        Tensor::ints(vec![edges.len().max(1), 2], edge_idx),
    );
    out.insert(
        "edge_type".into(),
        Tensor::ints(vec![edges.len().max(1)], edge_types),
    );
    out.insert(
        "edge_mask".into(),
        Tensor::bools(
            vec![edges.len().max(1)],
            (0..edges.len().max(1)).map(|i| i < edges.len()),
        ),
    );
    let mut pair_idx = Vec::new();
    for pair in pairs {
        let pair = pair.as_array().context("pair row")?;
        ensure!(pair.len() == 2, "pair width");
        for v in pair {
            let k = integer(v)?;
            ensure!(
                k >= 0 && usize::try_from(k)? < n,
                "unrepresented pair endpoint"
            );
            pair_idx.push(k);
        }
    }
    pair_idx.resize(pairs.len().max(1) * 2, 0);
    out.insert(
        "pair_index".into(),
        Tensor::ints(vec![pairs.len().max(1), 2], pair_idx),
    );
    let head = integer(&features["head"])?;
    let decision = boolean(&features["is_decision"])?;
    let original_multi = array(target, "multi_mask")?;
    let original_free = array(target, "free_mask")?;
    for (prefix, size, active) in [("cand", c, head == 1), ("pair", pairs.len(), head == 2)] {
        for (suffix, original) in [
            ("target_mask", original_multi),
            ("free_mask", original_free),
        ] {
            let mut values = if active && decision {
                mask(original, size)?
            } else {
                vec![0.; size]
            };
            values.resize(size.max(1), 0.);
            out.insert(
                format!("{prefix}_{suffix}"),
                Tensor::floats(vec![size.max(1)], values),
            );
        }
    }
    let picks = array(target, "pick_sequence")?;
    let mut pick_values = picks.iter().map(integer).collect::<Result<Vec<_>>>()?;
    let pick_count = pick_values.len();
    pick_values.resize(pick_count.max(1), 0);
    out.insert(
        "pick_sequence".into(),
        Tensor::ints(vec![pick_count.max(1)], pick_values),
    );
    out.insert(
        "pick_sequence_mask".into(),
        Tensor::bools(
            vec![pick_count.max(1)],
            (0..pick_count.max(1)).map(|i| i < pick_count),
        ),
    );
    let has_lp = boolean(&provenance["has_logprob"])?;
    let candidate = boolean(&row["actor_eligible"])?;
    let genuine = candidate
        && match head {
            0 => c > 1,
            1 | 2 => original_free.iter().any(|v| v == 1),
            3 => false,
            _ => anyhow::bail!("unsupported head"),
        };
    for (name, value) in [
        ("head", head),
        ("single_idx", integer(&target["single_idx"])?),
        (
            "source_kind",
            if provenance["source"] == "Net" { 0 } else { 2 },
        ),
        ("thinking_steps", integer(&provenance["thinking_steps"])?),
        ("entity_count", i64::try_from(n)?),
        ("candidate_count", i64::try_from(c)?),
        ("pair_count", i64::try_from(pairs.len())?),
        ("pick_sequence_length", i64::try_from(pick_count)?),
    ] {
        out.insert(name.into(), Tensor::ints(vec![], [value]));
    }
    for (name, value) in [
        ("scalar_target", real(&target["scalar_norm"])?),
        ("behavior_logprob", real(&provenance["behavior_logprob"])?),
        ("temperature", real(&provenance["temperature"])?),
        (
            "training_utility",
            real(&row["returns"]["training_utility"])?,
        ),
    ] {
        out.insert(name.into(), Tensor::floats(vec![], [value]));
    }
    for (name, value) in [
        ("is_decision", decision),
        ("has_logprob", has_lp),
        ("actor_candidate", candidate),
        ("actor_mask", genuine),
        ("training_terminal", boolean(&row["training_terminal"])?),
        ("step_valid", true),
    ] {
        out.insert(name.into(), Tensor::bools(vec![], [value]));
    }
    Ok(out)
}
