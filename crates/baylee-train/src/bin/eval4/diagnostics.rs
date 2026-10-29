//! Bind non-policy diagnostics to the exact recorded accepted input.
use anyhow::{Context as _, ensure};
use serde_json::Value;

pub(super) fn bind_fallback_events(events: &mut [Value], record: &[u8]) -> anyhow::Result<()> {
    for event in events {
        if event["accepted"] != true {
            continue;
        }
        let offset = usize::try_from(
            event["record_byte_offset"]
                .as_u64()
                .context("fallback record offset")?,
        )?;
        let line = record
            .get(offset..)
            .context("fallback outside record")?
            .split(|&byte| byte == b'\n')
            .next()
            .context("fallback record line")?;
        let input: Value = serde_json::from_slice(line)?;
        ensure!(
            input["kind"] == "input"
                && input["seat"] == event["seat"]
                && input["action"] == event["action"],
            "fallback diagnostic differs from recorded action"
        );
        event["record_input_n"] = input["n"].clone();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn diagnostic_binds_the_accepted_input_and_rejects_action_changes() {
        let record =
            b"prefix\n{\"kind\":\"input\",\"n\":12,\"seat\":1,\"action\":\"PassPriority\"}\n";
        let event =
            json!({"record_byte_offset":7,"accepted":true,"seat":1,"action":"PassPriority"});
        let mut events = vec![event.clone()];
        bind_fallback_events(&mut events, record).unwrap();
        assert_eq!(events[0]["record_input_n"], 12);
        events[0]["action"] = json!("CancelCast");
        assert!(bind_fallback_events(&mut events, record).is_err());
        let mut rejected = vec![json!({"accepted":false,"record_byte_offset":999})];
        bind_fallback_events(&mut rejected, record).unwrap();
        assert!(rejected[0]["record_input_n"].is_null());
    }
}
