//! Codex always sends `--output-schema` in strict mode. Unlike the other
//! transports, it requires closed objects and every property to be present.
//! Unused decision fields are represented by null, which the reader treats
//! as absent. See `OpenAI`'s structured-outputs guide and `docs/llm-seat.md`.

use crate::llm::prompt;
use serde_json::{Value, json};

/// The answer schema of a seat with `extras`, adapted to Codex's strict
/// response format.
pub(super) fn answer(extras: prompt::Extras) -> Value {
    let mut schema = prompt::answer_schema(extras);
    close(&mut schema);
    schema
}

/// Closes each object in our schema, including objects inside arrays.
fn close(schema: &mut Value) {
    if let Some(items) = schema.get_mut("items") {
        close(items);
    }
    let required = schema["required"].as_array().cloned().unwrap_or_default();
    if let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) {
        for (name, field) in &mut *properties {
            close(field);
            if !required.contains(&Value::String(name.clone())) {
                *field = json!({"anyOf": [field.take(), {"type": "null"}]});
            }
        }
        let names: Vec<_> = properties.keys().cloned().collect();
        schema["required"] = json!(names);
        schema["additionalProperties"] = Value::Bool(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrator::Decision;

    #[test]
    fn every_object_is_closed_and_requires_all_its_fields() {
        fn check(value: &Value) {
            match value {
                Value::Object(map) => {
                    if value["type"] == "object" {
                        assert_eq!(value["additionalProperties"], false);
                        let properties = value["properties"].as_object().unwrap();
                        let required = value["required"].as_array().unwrap();
                        assert_eq!(required.len(), properties.len());
                        for name in properties.keys() {
                            assert!(required.contains(&json!(name)), "{name}");
                        }
                    }
                    for child in map.values() {
                        check(child);
                    }
                }
                Value::Array(values) => values.iter().for_each(check),
                _ => {}
            }
        }
        let schema = answer(prompt::Extras::default());
        check(&schema);
        assert_eq!(schema["properties"]["ask"]["type"], "string");
        for (name, field) in schema["properties"].as_object().unwrap() {
            if name != "ask" {
                assert_eq!(field["anyOf"][1], json!({"type": "null"}), "{name}");
            }
        }
        assert_eq!(
            schema["properties"]["until"]["anyOf"][0]["enum"],
            prompt::answer_schema(prompt::Extras::default())["properties"]["until"]["enum"]
        );
    }

    #[test]
    fn unused_null_fields_preserve_decisions_and_concessions() {
        for sparse in [
            json!({"ask":"q12", "pick":["p"], "plan":["pass"]}),
            json!({"ask":"q13", "attacks":[{"attacker":"#1", "at":"P2"}]}),
            json!({"ask":"q14", "concede":"No way to survive."}),
        ] {
            let mut full = sparse.clone();
            for name in answer(prompt::Extras::default())["properties"]
                .as_object()
                .unwrap()
                .keys()
            {
                full.as_object_mut()
                    .unwrap()
                    .entry(name)
                    .or_insert(Value::Null);
            }
            assert_eq!(
                Decision::from_json_answer(&full).unwrap(),
                Decision::from_json_answer(&sparse).unwrap()
            );
        }
    }
}
