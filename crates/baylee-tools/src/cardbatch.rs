//! Read an external model envelope without treating its prose as a verdict.
use std::{fs, path::Path};

use anyhow::{Result, bail};
use serde_json::Value;

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(v) => *v,
        Value::Number(v) => v.as_f64().is_some_and(|n| n.abs() > 0.0),
        Value::String(v) => !v.is_empty(),
        Value::Array(v) => !v.is_empty(),
        Value::Object(v) => !v.is_empty(),
    }
}

fn response(raw: &Value) -> Option<&Value> {
    ["response", "result", "output"]
        .iter()
        .filter_map(|key| raw.get(key))
        .find(|v| truthy(v))
}

fn payload(raw: &Value) -> Option<&Value> {
    if !raw.is_object() {
        return None;
    }
    raw.get("structured_output")
        .filter(|v| v.is_object())
        .or_else(|| response(raw).filter(|v| v.is_object()))
}

fn envelope(raw: &Value) -> Value {
    if let Some(value) = payload(raw) {
        return value.clone();
    }
    let text = response(raw).and_then(Value::as_str).unwrap_or_default();
    text.rfind("{\"slug\"")
        .and_then(|start| {
            serde_json::Deserializer::from_str(&text[start..])
                .into_iter::<Value>()
                .next()
                .and_then(Result::ok)
                .filter(Value::is_object)
        })
        .unwrap_or(Value::Null)
}

fn cell(value: &Value) -> String {
    if !truthy(value) {
        return "-".into();
    }
    let value = match value {
        Value::Null | Value::Bool(false) => String::new(),
        Value::Number(n) if n.as_i64() == Some(0) => String::new(),
        Value::String(s) => s.clone(),
        _ => value.to_string(),
    };
    let text = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() { "-".into() } else { text }
}

pub fn run(_root: &Path, args: &[String]) -> Result<()> {
    let [path, question, rest @ ..] = args else {
        bail!("usage: cardbatch <verdict.json> status|row [slug name]");
    };
    // An unreadable envelope must never be evidence that a card is correct.
    let data = fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .map_or(Value::Null, |v| envelope(&v));
    match question.as_str() {
        "status" if rest.is_empty() => {
            let status = data["status"].as_str().unwrap_or_default();
            println!(
                "{}",
                if matches!(status, "implemented" | "partial" | "refused") {
                    status
                } else {
                    "refused"
                }
            );
        }
        "row" if rest.len() == 2 => {
            let fields = [
                Value::String(rest[0].clone()),
                Value::String(rest[1].clone()),
                data.get("status")
                    .cloned()
                    .unwrap_or_else(|| Value::String("unreadable".into())),
                data["oracle_sentence"].clone(),
                data["cannot_say"].clone(),
                data["nearest_existing"].clone(),
            ];
            println!("{}", fields.iter().map(cell).collect::<Vec<_>>().join("\t"));
        }
        _ => bail!("usage: cardbatch <verdict.json> status|row [slug name]"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn structured_answer_beats_narration_and_final_appended_answer_is_used() {
        let value = json!({"structured_output":{"status":"implemented"},"response":"{\"slug\":\"x\",\"status\":\"refused\"}"});
        assert_eq!(envelope(&value)["status"], "implemented");
        let value = json!({"response":"old {\"slug\":\"x\",\"status\":\"refused\"}\n final {\"slug\":\"x\",\"status\":\"partial\"} trailing narration"});
        assert_eq!(envelope(&value)["status"], "partial");
        assert!(envelope(&json!({"response":"not a verdict"})).is_null());
        assert!(envelope(&json!([])).is_null());
        assert!(envelope(&json!({"response":"{\"slug\":broken"})).is_null());
    }
    #[test]
    fn tsv_cannot_inject_rows_or_columns() {
        assert_eq!(cell(&json!("hello\nworld\tmore")), "hello world more");
        assert_eq!(cell(&Value::Null), "-");
    }
}

#[cfg(test)]
mod fallback_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_or_false_narration_does_not_hide_the_actual_result() {
        for empty in [
            Value::Null,
            json!(false),
            json!(0),
            json!(0.0),
            json!(""),
            json!([]),
            json!({}),
        ] {
            let raw = json!({"response":empty,"result":{"slug":"x","status":"implemented"}});
            assert_eq!(envelope(&raw)["status"], "implemented");
            assert_eq!(cell(&empty), "-");
        }
        // An explicitly supplied structured object is authoritative, even empty.
        assert_eq!(
            envelope(&json!({"structured_output":{},"result":{"status":"implemented"}})),
            json!({})
        );
    }
}
