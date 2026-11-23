//! What a provider may answer that no provider should: every body is read or
//! refused, and none of them stops the seat.

use super::*;

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

    /// A value of at most `depth` levels made of the words the readers look
    /// for, each under names and in places it does not belong.
    fn value(&mut self, depth: usize) -> Value {
        const WORDS: [&str; 24] = [
            "choices",
            "message",
            "content",
            "tool_calls",
            "function",
            "arguments",
            "name",
            "id",
            "usage",
            "prompt_tokens",
            "prompt_tokens_details",
            "cached_tokens",
            "type",
            "text",
            "tool_use",
            "thinking",
            "input",
            "stop_reason",
            "finish_reason",
            "error",
            "code",
            "{\"a\":1}",
            "}{",
            "é{",
        ];
        let sides = if depth == 0 { 5 } else { 8 };
        match self.roll(sides) {
            0 => Value::Null,
            1 => json!(self.roll(2) == 0),
            2 => json!([0, 1, -1, i64::MIN][self.roll(4)]),
            3 => json!(1.5),
            4 => json!(WORDS[self.roll(WORDS.len())]),
            5 => (0..self.roll(3)).map(|_| self.value(depth - 1)).collect(),
            _ => {
                let mut object = serde_json::Map::new();
                for _ in 0..self.roll(5) {
                    let key = WORDS[self.roll(WORDS.len())];
                    object.insert(key.into(), self.value(depth - 1));
                }
                Value::Object(object)
            }
        }
    }
}

const MODES: [AnswerMode; 3] = [AnswerMode::Tools, AnswerMode::Json, AnswerMode::JsonSchema];

#[test]
fn a_body_in_any_shape_is_read_or_refused_and_never_stops_the_seat() {
    let mut dice = Dice(3);
    let (mut read, mut refused) = (0, 0);
    for _ in 0..4_000 {
        let body = json!({
            "choices": [{"message": dice.value(4), "finish_reason": dice.value(1)}],
            "usage": dice.value(3), "content": dice.value(3), "error": dice.value(2),
        });
        for mode in MODES {
            let reply = openai::parse(&body, mode).expect("a message is there");
            // The call made of a JSON answer, which no word above names.
            for call in reply.calls.iter().filter(|call| call.name == "decide") {
                assert!(mode.is_json() && call.input.is_object(), "{body}");
            }
        }
        match anthropic::parse(&body) {
            Ok(reply) => {
                assert_eq!(
                    reply.assistant["content"], body["content"],
                    "replayed as it came"
                );
                read += 1;
            }
            Err(_) => refused += 1,
        }
        let _ = (
            openai::error_message(&body),
            anthropic::error_message(&body),
        );
        let odd = dice.value(4);
        for mode in MODES {
            let _ = openai::parse(&odd, mode);
        }
        let _ = anthropic::parse(&odd);
    }
    assert!(read > 100 && refused > 100, "{read} {refused}");
}

#[test]
fn an_object_is_found_in_prose_and_nothing_else_is_taken_for_one() {
    let a = json!({"a": 1});
    for text in [
        r#"{"a":1}"#,
        "  {\"a\":1}\n",
        "```json\n{\"a\":1}\n```",
        "I choose {\"a\":1}.",
        "é{\"a\":1}é",
        "{\"a\":1} because {it} wins",
        "{oops} then {\"a\":1}",
        "}{\"a\":1}",
    ] {
        assert_eq!(openai::json_object(text), Some(a.clone()), "{text}");
    }
    for text in [
        "", "{", "}", "}{", "[1]", "\"{}", "null", "[[]]", "{a:1}", "{\"a\":", "é",
    ] {
        assert_eq!(openai::json_object(text), None, "{text}");
    }
    // Nested or repeated far past any answer: an end, and not a blown stack.
    assert_eq!(openai::json_object(&"{".repeat(200_000)), None);
    assert_eq!(openai::json_object(&"{\"a\":".repeat(100_000)), None);
}

#[test]
fn a_model_is_named_by_characters_a_model_id_has() {
    for good in [
        "anthropic:claude-x",
        "openai:org/model-1.0:free",
        "cli:claude",
    ] {
        assert!(Spec::parse(good).expect("a mind's").is_ok(), "{good}");
    }
    for bad in [
        "openai",
        "openai:",
        "openai: ",
        "cli",
        "cli:",
        "cli:nothing",
        "openai:a b",
        "openai:a\nb",
        "openai:a\"b",
        "anthropic:ä",
        "anthropic:\u{202e}x",
        "cli:claude:--x y",
    ] {
        assert!(Spec::parse(bad).expect("a mind's").is_err(), "{bad}");
    }
    for other in ["", "house", "Anthropic:x", "openai2:x", " anthropic"] {
        assert!(Spec::parse(other).is_none(), "{other}");
    }
}

#[test]
fn a_chair_is_named_by_at_most_twelve_characters_a_seat_name_allows() {
    for model in [
        "anthropic:claude-opus-5-5-20261001",
        "openai:org/very-long-model-name-1.0:free",
        "openai:a/b/c",
        "openai:..",
        "openai:-x-",
        "cli:claude:opus",
        "cli:codex:gpt-6.1-sol",
    ] {
        let tag = Spec::parse(model).unwrap().unwrap().tag();
        let name = format!("LLM-{tag}");
        assert!((1..=12).contains(&tag.len()), "{model}: {tag}");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')),
            "{name}"
        );
        assert!(
            name.ends_with(|c: char| c.is_ascii_alphanumeric()),
            "{name}"
        );
    }
}

#[test]
fn a_cumulative_count_that_went_back_counts_from_there() {
    let of = |input, output, cache_read| Usage {
        input,
        output,
        cache_read,
        ..Usage::default()
    };
    assert_eq!(of(10, 20, 30).since(of(4, 20, 0)), of(6, 0, 30));
    // A tool that began counting again: what it counts now, never a number
    // below nothing.
    assert_eq!(of(3, 25, 1).since(of(4, 20, 2)), of(3, 5, 1));
}

#[test]
fn a_call_that_may_have_been_billed_is_held_until_it_is_known() {
    let settings = {
        let mut settings = Settings::new(&Spec::parse("anthropic").unwrap().unwrap());
        settings.hard_limit = true;
        settings.spend_tokens = 100;
        settings.spend_usd = None;
        settings.spend_calls = Some(4);
        settings
    };
    let worst = |tokens| Worst { tokens, usd: None };
    let mut tally = Tally::default();
    assert!(tally.hold(worst(60), &settings).is_ok());
    assert!(tally.hold(worst(41), &settings).is_err(), "60 are out");
    assert!(tally.hold(worst(u64::MAX), &settings).is_err());
    assert!(tally.hold(worst(40), &settings).is_ok());
    assert_eq!(tally.spend_tokens(), 100);
    // Turned away for nothing: what was held is free again.
    tally.back(worst(40), Err(false), None);
    assert_eq!((tally.spend_tokens(), tally.failed), (60, 1));
    // Answered, for less than the worst.
    let usage = Usage {
        input: 5,
        output: 5,
        ..Usage::default()
    };
    tally.back(worst(60), Ok(usage), None);
    assert_eq!(tally.spend_tokens(), 10);
    // Nobody knows what it cost: it counts as the worst.
    assert!(tally.hold(worst(50), &settings).is_ok());
    tally.back(worst(50), Err(true), None);
    assert_eq!(tally.spend_tokens(), 60);
    assert!(tally.spent_under(&settings).is_none());
    assert!(tally.hold(worst(1), &settings).is_ok());
    tally.back_at_worst(worst(1));
    assert_eq!((tally.calls, tally.out), (4, 0));
    let why = tally.hold(worst(1), &settings).expect_err("four calls");
    assert!(why.contains("calls"), "{why}");
    assert!(tally.spent_under(&settings).is_some());
    assert!(tally.spent);
}

#[test]
fn a_price_counts_every_kind_of_token_once() {
    let price = Price {
        input: 1.0,
        output: 10.0,
        cache_write: 100.0,
        cache_write_hour: 1_000.0,
        cache_read: 10_000.0,
    };
    let usage = Usage {
        input: 1_000_000,
        output: 2_000_000,
        cache_write: 5_000_000,
        cache_write_hour: 2_000_000,
        cache_read: 1_000_000,
    };
    let cost = usage.cost(price);
    assert!((cost - 12_321.0).abs() < 1e-6, "{cost}");
    assert_eq!(usage.total(), 9_000_000, "the hour's are of the writes");
}
