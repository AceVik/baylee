//! Reading provider answers against the current question.

use super::{AnswerMode, Decision, Menu, Read, Reply, Stop};

/// Reads a reply's first `decide` or `concede` call against `menu`.
pub(super) fn read(reply: &Reply, menu: &Menu, mode: AnswerMode) -> Result<Read, String> {
    let mut chosen = None;
    let mut others = Vec::new();
    for call in &reply.calls {
        let known = call.name == "decide" || call.name == "concede";
        if known && chosen.is_none() {
            chosen = Some(call);
        } else if !call.id.is_empty() {
            others.push(call.id.clone());
        }
    }
    let Some(call) = chosen else {
        return Err(match (mode, &reply.stop) {
            (_, Stop::MaxTokens) => "the reply ran out of tokens before it answered".into(),
            (AnswerMode::Tools, _) => "answer with one call of the decide tool".into(),
            (AnswerMode::Json | AnswerMode::JsonSchema, _) => {
                "answer with one JSON object and nothing else".into()
            }
        });
    };
    let decision = if mode.is_json() && call.id.is_empty() {
        Decision::from_json_answer(&call.input)?
    } else if call.name == "concede" {
        Decision::from_concede(&call.input)
    } else {
        Decision::from_decide(&call.input)?
    };
    let resolved = menu.resolve(&decision)?;
    Ok(Read {
        resolved,
        say: decision.say.filter(|s| !s.is_empty()),
        call: (!call.id.is_empty()).then(|| call.id.clone()),
        others,
    })
}
