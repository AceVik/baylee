use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Clone, Deserialize, Serialize)]
struct Tensor {
    shape: Vec<u64>,
    element_size: u64,
}
type Schema = BTreeMap<String, Tensor>;
fn product(shape: &[u64]) -> Result<u64> {
    shape.iter().try_fold(1u64, |a, b| {
        a.checked_mul(*b).context("tensor shape overflow")
    })
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).context("tensor byte estimate overflow")
}
fn mul(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b).context("tensor byte estimate overflow")
}
pub(super) fn estimate(value: &Value) -> Result<Value> {
    let (items, frame) = if value.get("frame_schemas").is_some() {
        segments(value)?
    } else {
        (
            serde_json::from_value(value["schemas"].clone())?,
            value["frame_peak_bytes"]
                .as_u64()
                .context("frame allocation peak")?,
        )
    };
    ensure!(!items.is_empty(), "batch schemas required");
    let mut collated = 0;
    let mut segment = 0;
    for (name, tensor) in &items[0] {
        ensure!(
            (1..=16).contains(&tensor.element_size),
            "invalid tensor scalar size"
        );
        let mut maxima = tensor.shape.clone();
        for item in &items {
            ensure!(
                item.len() == items[0].len(),
                "inconsistent tensor schema keys"
            );
            let other = item.get(name).context("missing tensor schema key")?;
            ensure!(
                other.shape.len() == maxima.len() && other.element_size == tensor.element_size,
                "inconsistent tensor rank or dtype"
            );
            for (a, b) in maxima.iter_mut().zip(&other.shape) {
                *a = (*a).max(*b);
            }
            segment = add(segment, mul(product(&other.shape)?, other.element_size)?)?;
        }
        collated = add(
            collated,
            mul(
                mul(u64::try_from(items.len())?, product(&maxima)?)?,
                tensor.element_size,
            )?,
        )?;
    }
    let unroll = value["unroll"].as_u64().context("unroll length")?;
    ensure!(unroll > 0, "positive unroll required");
    let peak = add(
        add(segment, mul(2, collated)?)?,
        mul(mul(2, frame)?, unroll)?,
    )?;
    let budget = value["budget_mib"].as_u64().context("tensor budget")?;
    ensure!(budget > 0, "positive tensor budget required");
    Ok(
        json!({"predicted_collated_bytes":collated,"conservative_tensor_peak_bytes":peak,"measure":peak<=mul(budget,1<<20)?}),
    )
}

fn segments(value: &Value) -> Result<(Vec<Schema>, u64)> {
    let input: Vec<Vec<Schema>> = serde_json::from_value(value["frame_schemas"].clone())?;
    let unroll = value["unroll"].as_u64().context("unroll")?;
    let mut schemas = Vec::new();
    let mut frame_peak = 0;
    for frames in input {
        ensure!(
            !frames.is_empty() && u64::try_from(frames.len())? <= unroll,
            "invalid segment frame count"
        );
        let mut schema = frames[0].clone();
        for frame in &frames {
            ensure!(
                frame.len() == schema.len(),
                "frame tensor schema keys differ"
            );
            let mut bytes = 0;
            for (key, tensor) in frame {
                let combined = schema.get_mut(key).context("frame tensor key missing")?;
                ensure!(
                    tensor.shape.len() == combined.shape.len()
                        && tensor.element_size == combined.element_size,
                    "frame rank/dtype differs"
                );
                for (a, b) in combined.shape.iter_mut().zip(&tensor.shape) {
                    *a = (*a).max(*b);
                }
                bytes = add(bytes, mul(product(&tensor.shape)?, tensor.element_size)?)?;
            }
            frame_peak = frame_peak.max(bytes);
        }
        for tensor in schema.values_mut() {
            tensor.shape.insert(0, unroll);
        }
        for (key, shape, size) in [
            ("step_valid", vec![unroll], 1),
            ("is_new_traj", vec![], 1),
            ("segment_index", vec![], 8),
        ] {
            ensure!(
                schema
                    .insert(
                        key.into(),
                        Tensor {
                            shape,
                            element_size: size
                        }
                    )
                    .is_none(),
                "reserved frame schema key"
            );
        }
        schemas.push(schema);
    }
    Ok((schemas, frame_peak))
}
