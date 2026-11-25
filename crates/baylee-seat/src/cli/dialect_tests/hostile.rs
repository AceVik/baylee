//! What a CLI may write that no CLI should: each line is read as something,
//! none stops the reader, and what a line says cannot lead out of the
//! directories a conversation is kept in.

use super::super::dialect::conversation_id;
use super::*;
use serde_json::json;

/// A fixed sequence (the 64-bit LCG of Knuth's MMIX), so a failure repeats.
struct Dice(u64);

impl Dice {
    fn roll(&mut self, sides: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap() % sides
    }

    fn word(&mut self) -> &'static str {
        const WORDS: [&str; 44] = [
            "type",
            "system",
            "init",
            "subtype",
            "result",
            "success",
            "error",
            "is_error",
            "session_id",
            "sessionID",
            "thread_id",
            "thread.started",
            "turn.completed",
            "turn.failed",
            "item.completed",
            "item",
            "agent_message",
            "text",
            "usage",
            "input_tokens",
            "output_tokens",
            "structured_output",
            "assistant",
            "message",
            "content",
            "tools",
            "mcp_servers",
            "apiKeySource",
            "permissionMode",
            "model",
            "event",
            "method",
            "params",
            "id",
            "jsonrpc",
            "update",
            "sessionUpdate",
            "step_finish",
            "part",
            "tokens",
            "../../etc/passwd",
            "|99999999999999999999",
            "{\"a\":1}",
            "é",
        ];
        WORDS[self.roll(WORDS.len())]
    }

    fn value(&mut self, depth: usize) -> Value {
        match self.roll(if depth == 0 { 5 } else { 9 }) {
            0 => Value::Null,
            1 => json!(self.roll(2) == 0),
            2 => json!([0, 7, -1, 1_000][self.roll(4)]),
            3 => json!(1.5),
            4 => json!(self.word()),
            5 => (0..self.roll(3)).map(|_| self.value(depth - 1)).collect(),
            _ => {
                let mut object = serde_json::Map::new();
                for _ in 0..=self.roll(5) {
                    object.insert(self.word().into(), self.value(depth - 1));
                }
                Value::Object(object)
            }
        }
    }
}

#[test]
fn a_line_in_any_shape_is_read_as_something_and_never_stops_the_reader() {
    let mut dice = Dice(5);
    let dialects = the_dialects();
    for round in 0..3_000 {
        let mut line = dice.value(4).to_string();
        // Half of them cut short, where a process died writing.
        if round % 2 == 0 {
            let at = (0..=dice.roll(line.len() + 1))
                .rev()
                .find(|at| line.is_char_boundary(*at))
                .unwrap_or(0);
            line.truncate(at);
        }
        for dialect in &dialects {
            read_all(dialect.as_ref(), &[&line, &line]);
        }
    }
    let long = "[".repeat(100_000);
    for dialect in &dialects {
        for line in ["", " ", "\0", "\u{feff}{}", "null", &long] {
            read_all(dialect.as_ref(), &[line]);
        }
    }
}

/// A conversation's name becomes an argument and a file's name, so one that
/// could be a flag or lead to another directory is no name.
#[test]
fn a_conversation_s_name_is_never_a_flag_or_a_path() {
    for bad in [
        "", "-x", "--resume", "a/b", "..", ".", "../a", "a\\b", "a b", "a\nb", "a\0b", "~",
        "$HOME", "é", "a;b", "*",
    ] {
        assert_eq!(conversation_id(bad), None, "{bad:?}");
    }
    let id = "0199a213-81c0-7800-8aa1-bbab2a035a53";
    assert_eq!(conversation_id(id).as_deref(), Some(id));
}

#[test]
fn a_line_that_names_a_bad_conversation_keeps_none() {
    for bad in ["../../etc/passwd", "--yolo", "a/b", ""] {
        let lines = [
            json!({"type": "system", "subtype": "init", "session_id": bad}),
            json!({"type": "thread.started", "thread_id": bad}),
            json!({"type": "step_start", "sessionID": bad, "part": {"sessionID": bad}}),
            json!({"event": "init", "session_id": bad}),
            json!({"jsonrpc": "2.0", "id": 1, "result": {"sessionId": bad}}),
        ];
        for dialect in the_dialects() {
            let mut wire = Wire::default();
            for line in &lines {
                let _ = dialect.read_event(&line.to_string(), &mut wire);
            }
            assert_eq!(wire.conversation, None, "{:?}: {bad}", dialect.tool());
        }
    }
}

#[test]
fn no_variable_that_holds_a_secret_is_passed_whatever_its_case() {
    for name in [
        "BAYLEE_AGENT_TOKEN",
        "baylee_x",
        "GH_TOKEN",
        "AWS_PROFILE",
        "Anthropic_Base_Url",
        "OPENAI_ORG",
        "MISTRAL_API_KEY",
        "xai_api_key",
        "NPM_TOKEN",
        "MY_SECRET_THING",
        "PGPASSWORD",
        "db_password_file",
        "SSH_AUTH_SOCK",
        "database_url",
    ] {
        assert!(forbidden(name), "{name}");
    }
    for name in [
        "PATH", "HOME", "LANG", "TERM", "TOKENS", "API_KEYS", "GHOST", "XAWS_X",
    ] {
        assert!(!forbidden(name), "{name}");
    }
}
