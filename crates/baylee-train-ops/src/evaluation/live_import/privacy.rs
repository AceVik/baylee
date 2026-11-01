use anyhow::{Result, ensure};
use serde_json::Value;

const FORBIDDEN: &[&str] = &[
    "game_id",
    "game_uuid",
    "account",
    "account_id",
    "account_uuid",
    "user_id",
    "username",
    "display_name",
    "email",
    "token",
    "session",
    "session_id",
    "session_token",
    "invite_key",
    "timestamp",
    "created_at",
];

pub(super) fn check(value: &Value, key: &str) -> Result<()> {
    match value {
        Value::Object(fields) if key == "Human" => ensure!(
            fields.values().all(|v| v.as_i64() == Some(0)),
            "Human account identifiers were not anonymized"
        ),
        Value::Object(fields) => {
            for (name, child) in fields {
                ensure!(
                    !FORBIDDEN.contains(&name.to_lowercase().as_str()),
                    "identity or timestamp field present"
                );
                ensure!(!uuid(name), "identifier in JSON key");
                check(child, name)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                check(child, key)?;
            }
        }
        Value::String(s) if key != "scryfall_id" => {
            ensure!(!uuid(s), "non-card UUID present");
            ensure!(!email(s), "address present");
            ensure!(!timestamp(s), "wall-clock timestamp present");
        }
        _ => {}
    }
    Ok(())
}

fn uuid(s: &str) -> bool {
    s.as_bytes().windows(36).any(|w| {
        w.iter().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                *b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
    })
}
fn timestamp(s: &str) -> bool {
    s.as_bytes().windows(16).any(|w| {
        w.iter().enumerate().all(|(i, b)| match i {
            4 | 7 => *b == b'-',
            10 => b"T ".contains(b),
            13 => *b == b':',
            _ => b.is_ascii_digit(),
        })
    })
}
fn email(s: &str) -> bool {
    s.split(char::is_whitespace).any(|word| {
        let segments: Vec<_> = word.split('@').collect();
        segments.windows(2).any(|pair| {
            !pair[0].is_empty()
                && pair[1]
                    .split_once('.')
                    .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn nested_identifiers_and_addresses_are_detected_but_card_ids_remain_allowed() {
        for v in [
            json!({"nested":{"email":"x"}}),
            json!({"x":"a@b.c"}),
            json!({"x":"2026-10-29T13:42"}),
            json!({"x":"prefix01a1728e-df52-7476-a918-efb7fc77fbe2suffix"}),
            json!({"Human":{"account":41}}),
        ] {
            assert!(check(&v, "").is_err());
        }
        check(&json!({"scryfall_id":"01a1728e-df52-7476-a918-efb7fc77fbe2","controller":{"Human":{"account":0}}}),"").unwrap();
    }
}
