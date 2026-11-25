use super::{
    game::{Receipt, StandardSession, without_seq},
    hash, json_file, lines, menu_body, number, read,
};
use anyhow::{Context, Result, ensure};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::record::{Line, Source};
use baylee_protocol::v1::envelope::Msg;
use baylee_seat::{
    mind::{GameContext, Request},
    narrator::{Act, Decision, Narrator, tap},
};
use baylee_view::{LogTail, PlayerView, ViewFrame};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
    time::Duration,
};

#[derive(Clone)]
struct Snapshot {
    view: PlayerView,
    pending: Pending,
    note: Option<baylee_engine::engine::DecisionNote>,
    public_after_n: Value,
    public_view_sha256: String,
}

pub(super) struct Seat {
    context: Arc<GameContext>,
    wanted: BTreeSet<u64>,
    snapshots: BTreeMap<u64, Snapshot>,
    bridge: Vec<Value>,
    provider: Vec<Value>,
    matches: BTreeMap<u64, Option<usize>>,
    next: BTreeMap<usize, u64>,
    responses: BTreeMap<usize, Value>,
    actions: Vec<Value>,
    transcript_sha256: String,
    provider_transcript_sha256: String,
}

impl Seat {
    pub(super) fn load(
        attempt: &Path,
        receipt: &Receipt,
        seat: usize,
        record: &[Line],
        context: &Arc<GameContext>,
    ) -> Result<Self> {
        let bridge_bytes = read(receipt.artifact(&format!("transcript_seat{seat}"))?)?;
        let provider_path = attempt.join(format!(
            "transcripts/{}-seat{seat}-mind.jsonl",
            receipt.game
        ));
        let provider_bytes = read(&provider_path)?;
        let bridge: Vec<Value> = lines(&bridge_bytes)?;
        let provider = lines(&provider_bytes)?;
        ensure!(
            bridge.iter().any(|row| row["event"] == "over"),
            "No seat GameOver"
        );
        let inputs: Vec<_> = record
            .iter()
            .filter_map(|entry| match entry {
                Line::Input {
                    n,
                    seat: actor,
                    by: Source::Seat,
                    action,
                    ..
                } if usize::from(*actor) == seat => Some((*n, json!(action))),
                _ => None,
            })
            .collect();
        let provenance = json_file(receipt.artifact(&format!("executed_provenance_seat{seat}"))?)?;
        let matches = align(&bridge, &inputs, &provenance)?;
        let next = matches
            .iter()
            .filter_map(|(n, index)| index.map(|i| (i, *n)))
            .collect();
        let wanted = bridge
            .iter()
            .filter(|row| row["event"] == "asked")
            .filter_map(|row| row["seq"].as_u64())
            .collect();
        Ok(Self {
            context: Arc::clone(context),
            wanted,
            snapshots: BTreeMap::new(),
            bridge,
            provider,
            matches,
            next,
            responses: BTreeMap::new(),
            actions: Vec::new(),
            transcript_sha256: hash(&bridge_bytes),
            provider_transcript_sha256: hash(&provider_bytes),
        })
    }

    pub(super) fn capture(&mut self, session: &mut StandardSession, public: &Value) -> Result<()> {
        if self.wanted.contains(&session.seq())
            && let Some(snapshot) = self.snapshot(session, public)?
        {
            self.snapshots.insert(session.seq(), snapshot);
        }
        Ok(())
    }

    fn snapshot(&self, session: &mut StandardSession, public: &Value) -> Result<Option<Snapshot>> {
        let mut view = None;
        let mut pending = None;
        let mut note = None;
        for envelope in session.snapshot(self.context.seat) {
            match envelope.msg {
                Some(Msg::StateDelta(delta)) => view = ViewFrame::whole(&delta.view_frame),
                Some(Msg::ChoiceRequest(choice)) => {
                    pending = Some(baylee_core::wire::decode(&choice.pending)?);
                    if !choice.context.is_empty() {
                        note = Some(baylee_core::wire::decode(&choice.context)?);
                    }
                }
                _ => {}
            }
        }
        let Some(pending) = pending else {
            return Ok(None);
        };
        let view: PlayerView = view.context("Missing active seat view")?;
        ensure!(view.seq == session.seq() && view.seat == self.context.seat);
        Ok(Some(Snapshot {
            view,
            pending,
            note,
            public_after_n: public["n"].clone(),
            public_view_sha256: hash(&serde_json::to_vec(&public["json"])?),
        }))
    }

    pub(super) fn verify_input(
        &mut self,
        session: &mut StandardSession,
        n: u64,
        action: &PlayerAction,
    ) -> Result<()> {
        let Some(send_index) = self.matches.get(&n).context("Missing provenance entry")? else {
            self.actions
                .push(json!({"input_n":n, "origin":"ambiguous", "action":action}));
            return Ok(());
        };
        let send_index = *send_index;
        let row = &self.bridge[send_index];
        let question = number(row, "question")?;
        let base =
            json!({"question":question, "input_n":n, "send_line":send_index + 1, "action":action});
        let name = source(row);
        let mut result = base;
        if name == "llm" {
            self.verify_mind(session, n, action, send_index, &mut result)?;
        } else {
            result["origin"] = json!(name);
        }
        self.actions.push(result);
        Ok(())
    }

    fn verify_mind(
        &mut self,
        session: &mut StandardSession,
        n: u64,
        action: &PlayerAction,
        send_index: usize,
        result: &mut Value,
    ) -> Result<()> {
        let row = &self.bridge[send_index];
        let question = number(row, "question")?;
        let note: Value = serde_json::from_str(row["note"].as_str().context("Missing mind note")?)?;
        if note["plan"].is_string() && row["model_ms"] == 0 && row["thinking"].is_null() {
            result["origin"] = json!("local_mind");
            return Ok(());
        }
        let provider_matches: Vec<_> = self
            .provider
            .iter()
            .enumerate()
            .filter(|(_, provider)| {
                provider["question"] == question
                    && provider["error"].is_null()
                    && provider["reply"]["text"].is_string()
                    && provider["reply"]["text"] == row["thinking"]
                    && provider["reply"]["usage"] == note["tokens"]
            })
            .collect();
        let [(reply_index, provider)] = provider_matches.as_slice() else {
            result["origin"] = json!("provider_uncertain");
            result["candidate_reply_count"] = json!(provider_matches.len());
            return Ok(());
        };
        let reply_index = *reply_index;
        let proof = self.prove(session, n, action, send_index, question, provider, &note);
        let response = match proof {
            Ok((execution, candidate)) => {
                result["origin"] = json!("provider");
                json!({"reply_line":reply_index + 1,"question":question, "execution":execution,
                    "direct_label_candidate":candidate, "status":"verified_execution"})
            }
            Err(reason) => {
                result["origin"] = json!("provider_uncertain");
                json!({"reply_line":reply_index + 1,"question":question,"execution":null,
                    "direct_label_candidate":false,"status":"matched_send_only",
                    "detail":reason.to_string(), "input_n":n,"send_line":send_index + 1})
            }
        };
        ensure!(
            self.responses.insert(reply_index, response).is_none(),
            "One provider response matches multiple executed bridge sends"
        );
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Bind a native execution to its original request coordinates"
    )]
    fn prove(
        &self,
        session: &mut StandardSession,
        n: u64,
        action: &PlayerAction,
        send_index: usize,
        question: u64,
        provider: &Value,
        note: &Value,
    ) -> Result<(Value, bool)> {
        let (ask_index, ask) = self.bridge[..send_index]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, row)| row["event"] == "asked" && row["question"] == question)
            .context("No bridge ask before send")?;
        let seq = number(ask, "seq")?;
        let snapshot = self
            .snapshots
            .get(&seq)
            .context("No native seat view at bridge ask sequence")?;
        let request = Request {
            context: Arc::clone(&self.context),
            question,
            view: snapshot.view.clone(),
            pending: snapshot.pending.clone(),
            log: LogTail::default(),
            budget: Duration::from_millis(number(ask, "budget_ms")?),
            retry: None,
            continuing: ask["continuing"] == true,
            held: None,
            note: snapshot.note.clone(),
        };
        ensure!(
            provider["turn"] == request.view.turn
                && ask["turn"] == request.view.turn
                && ask["step"] == json!(request.view.step),
            "Turn or step mismatch at ask"
        );
        let answer = &provider["reply"]["answer"];
        ensure!(
            answer["ask"] == format!("q{question}"),
            "Provider answer names a different question"
        );
        let wake = Narrator::new(&request.context).wake(&request, &[], None);
        let prompt = menu_body(provider["message"].as_str().context("No provider prompt")?)?;
        let native = menu_body(&wake.text)?;
        ensure!(
            native == prompt,
            "Provider legal menu differs from native replay"
        );
        let decision = Decision::from_json_answer(answer).map_err(anyhow::Error::msg)?;
        let resolved = wake.menu.resolve(&decision).map_err(anyhow::Error::msg)?;
        ensure!(
            &resolved.act.first() == action && note["chose"] == resolved.label,
            "Resolved choice and accepted action differ"
        );
        let actual = self
            .snapshot(session, &Value::Null)?
            .context("No legal question immediately before accepted input")?;
        let same_context = without_seq(json!(actual.view)) == without_seq(json!(snapshot.view))
            && baylee_core::wire::encode(&actual.pending)
                == baylee_core::wire::encode(&snapshot.pending);
        let payment = self.payment_chain(n, &resolved.act);
        let directly = payment.is_null();
        let offered = wake.menu.offered().len();
        let unique_pass = matches!(action, PlayerAction::PassPriority) && offered <= 1;
        let same_seq = self.bridge[send_index]["seq"] == seq;
        let direct_candidate = directly
            && same_context
            && same_seq
            && !unique_pass
            && !matches!(action, PlayerAction::Concede | PlayerAction::OfferDraw)
            && !action.is_automation_setting();
        Ok((
            json!({
                "input_n":n, "ask_line":ask_index + 1, "send_line":send_index + 1,
                "ask_seq":seq, "send_seq":self.bridge[send_index]["seq"], "execution_pre_seq":session.seq(),
                "kind":ask["kind"], "action":action, "direct":directly, "same_context":same_context,
                "menu_items":offered, "menu_sha256":hash(native.as_bytes()),
                "seat_view_sha256":hash(&serde_json::to_vec(&json!(snapshot.view))?),
                "public_after_n":snapshot.public_after_n,"public_view_sha256":snapshot.public_view_sha256,
                "payment_chain":payment
            }),
            direct_candidate,
        ))
    }

    fn payment_chain(&self, n: u64, act: &Act) -> Value {
        let Act::Taps { steps, then } = act else {
            return Value::Null;
        };
        let Some(first) = steps.first() else {
            return Value::Null;
        };
        let mut expected = Vec::new();
        expected.extend(
            first
                .color
                .into_iter()
                .chain(first.second)
                .map(PlayerAction::ChooseColor),
        );
        for step in &steps[1..] {
            expected.push(tap(step));
            expected.extend(
                step.color
                    .into_iter()
                    .chain(step.second)
                    .map(PlayerAction::ChooseColor),
            );
        }
        expected.push(then.clone());
        let mut local = Vec::new();
        for ((follow_n, index), action) in self
            .matches
            .range((std::ops::Bound::Excluded(n), std::ops::Bound::Unbounded))
            .zip(&expected)
        {
            let Some(index) = index else {
                break;
            };
            let row = &self.bridge[*index];
            if row["by"] != "mind"
                || row["model_ms"] != 0
                || row["thinking"].is_string()
                || row["action"] != json!(action)
            {
                break;
            }
            local.push(json!({"input_n":follow_n, "send_line":index + 1, "question":row["question"], "action":action}));
        }
        json!({"chosen_action":then, "completed":local.len() == expected.len(),
            "expected_local_steps":expected.len(), "verified_local_steps":local})
    }

    pub(super) fn report(mut self) -> Result<Value> {
        let keys = [
            "input",
            "output",
            "cache_read",
            "cache_write",
            "cache_write_hour",
        ];
        let mut total = BTreeMap::new();
        let mut missing_usage = 0;
        let mut answers = Vec::new();
        let mut provider_errors = Vec::new();
        for (i, row) in self.provider.iter().enumerate() {
            if !row["reply"].is_object() {
                if row["error"].is_string() {
                    provider_errors.push(json!({"line":i + 1,"question":row["question"]}));
                }
                continue;
            }
            let usage = &row["reply"]["usage"];
            if usage.is_object() {
                for key in keys {
                    *total.entry(key).or_insert(0_u64) += usage[key].as_u64().unwrap_or(0);
                }
            } else {
                missing_usage += 1;
            }
            let mut report = self.responses.remove(&i).unwrap_or_else(|| {
                json!({
                    "reply_line":i + 1, "question":row["question"], "execution":null,
                    "direct_label_candidate":false, "status":"no_verified_execution"
                })
            });
            report["usage"] = usage.clone();
            answers.push(report);
        }
        ensure!(
            self.responses.is_empty(),
            "An execution lacks its original provider row"
        );
        let mut origins = BTreeMap::new();
        for action in &self.actions {
            *origins
                .entry(action["origin"].as_str().context("Missing action origin")?)
                .or_insert(0_usize) += 1;
        }
        let tokens = |key| total.get(key).copied().unwrap_or(0);
        Ok(json!({
            "seat":self.context.seat.get(),"seat_transcript_sha256":self.transcript_sha256,
            "provider_transcript_sha256":self.provider_transcript_sha256,
            "accepted_socket_inputs":self.matches.len(),"uniquely_matched_socket_inputs":self.next.len(),
            "replies":answers.len(),"provider_errors":provider_errors,"replies_without_usage":missing_usage,
            "verified_replies":answers.iter().filter(|a| a["execution"].is_object()).count(),
            "direct_label_candidates":answers.iter().filter(|a| a["direct_label_candidate"] == true).count(),
            "tokens":total,"tokens_total":tokens("input") + tokens("output") + tokens("cache_read") + tokens("cache_write"),
            "accepted_input_origins":origins, "accepted_inputs":self.actions, "responses":answers
        }))
    }
}

fn source(row: &Value) -> &'static str {
    if row["event"] == "standing" {
        return "standing";
    }
    match row["by"].as_str() {
        Some("mind") => "llm",
        Some("house") => "house",
        Some("standing") => "standing",
        _ => "other",
    }
}

fn align(
    bridge: &[Value],
    inputs: &[(u64, Value)],
    provenance: &Value,
) -> Result<BTreeMap<u64, Option<usize>>> {
    let mut sends = Vec::<usize>::new();
    for (index, row) in bridge.iter().enumerate() {
        match row["event"].as_str() {
            Some("answered" | "standing") => sends.push(index),
            Some("refused") => {
                if let Some(old) = sends.iter().rposition(|i| {
                    bridge[*i]["question"] == row["question"]
                        && bridge[*i]["action"] == row["action"]
                }) {
                    sends.remove(old);
                }
            }
            _ => {}
        }
    }
    let mut earliest = Vec::with_capacity(inputs.len());
    let mut cursor = 0;
    for (_, action) in inputs {
        let delta = sends[cursor..]
            .iter()
            .position(|i| &bridge[*i]["action"] == action)
            .context("Accepted actions lack ordered sent subsequence")?;
        cursor += delta;
        earliest.push(cursor);
        cursor += 1;
    }
    let mut latest = vec![0; inputs.len()];
    cursor = sends.len();
    for (i, (_, action)) in inputs.iter().enumerate().rev() {
        cursor = sends[..cursor]
            .iter()
            .rposition(|i| &bridge[*i]["action"] == action)
            .context("Accepted actions lack ordered sent suffix")?;
        latest[i] = cursor;
    }
    let claims = provenance["inputs"]
        .as_array()
        .context("Missing ordered provenance")?;
    ensure!(claims.len() == inputs.len() && provenance["ppo_eligible"] == false);
    let mut map = BTreeMap::new();
    for (i, (n, action)) in inputs.iter().enumerate() {
        let start = i
            .checked_sub(1)
            .map_or(0, |previous| earliest[previous] + 1);
        let end = latest.get(i + 1).copied().unwrap_or(sends.len());
        let matching: Vec<_> = sends[start..end]
            .iter()
            .copied()
            .filter(|j| &bridge[*j]["action"] == action)
            .collect();
        let origins: Vec<_> = ["llm", "house", "standing", "other"]
            .into_iter()
            .filter(|kind| matching.iter().any(|j| source(&bridge[*j]) == *kind))
            .collect();
        let claim = &claims[i];
        ensure!(
            claim["record_input_n"] == *n
                && claim["action"] == *action
                && claim["possible_sources"] == json!(origins)
                && claim["possible_send_count"] == matching.len()
                && claim["ambiguous"] == (origins.len() > 1)
                && claim["policy_target_admitted"] == false,
            "Independent ordered provenance differs at input {n}"
        );
        map.insert(
            *n,
            match matching.as_slice() {
                [only] => Some(*only),
                _ => None,
            },
        );
    }
    Ok(map)
}
