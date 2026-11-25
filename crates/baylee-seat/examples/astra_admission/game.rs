use super::{
    ASTRA, Pins, TERRA, hash, json_file, lines, objects::Objects, read, seat::Seat, unpack,
};
use anyhow::{Context, Result, ensure};
use baylee_core::{
    ids::{CardIndex, PlayerId},
    preset::GamePreset,
};
use baylee_engine::{choice::Pending, engine::NoLoopDetection};
use baylee_gamehost::{
    Session,
    record::{Line, MindKind, Source},
};
use baylee_seat::{
    deck::Deck,
    mind::{DeckList, GameContext},
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

pub(super) type StandardSession = Session<NoLoopDetection>;

pub(super) struct Receipt {
    pub(super) game: String,
    pub(super) technical: Value,
    pub(super) request: Value,
    pub(super) artifacts: BTreeMap<String, PathBuf>,
}

impl Receipt {
    #[allow(
        clippy::too_many_lines,
        reason = "The receipt is checked field by field, in its order"
    )]
    pub(super) fn load(attempt: &Path) -> Result<Self> {
        let accepted = json_file(&attempt.join("receipt.json"))?;
        let technical = json_file(&attempt.join("technical-result.json"))?;
        let request = json_file(&attempt.join("request.json"))?;
        ensure!(
            accepted["request"] == request
                && technical["bc_only"] == true
                && technical["ppo_eligible"] == false
                && request["numbers_only"] != true,
            "Invalid TRAIN receipt"
        );
        ensure!(
            [
                json!([ASTRA, ASTRA]),
                json!([ASTRA, TERRA]),
                json!([TERRA, ASTRA])
            ]
            .contains(&request["models"]),
            "Unsupported ordered TRAIN model pair"
        );
        let room = json_file(&attempt.join("room-state.json"))?;
        ensure!(
            room["status"] == "closed"
                && room["start_requested"] == true
                && room["proof"]
                    == "Complete official exported End for this owned game; transcript and model admission checked separately",
            "Original collector did not prove terminal closure"
        );
        let game = room["game_id"]
            .as_str()
            .context("Missing game ID")?
            .to_owned();
        ensure!(
            game.len() == 36
                && game
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() || byte == b'-')
        );
        let mut artifacts = BTreeMap::new();
        if technical["status"] == "failed" {
            ensure!(
                accepted["status"] == "error"
                    && technical["error_message"] == "Full engine replay failed: Unbuildable",
                "Only the original loop-variant mismatch can be independently recovered"
            );
            for (role, name) in [
                ("anonymized_record", "official-export/000001.jsonl.gz"),
                ("replay_tape", "public-replay.jsonl.gz"),
                ("room_setup_receipt", "room-setup-receipt.json"),
                ("input_receipt", "input-receipt.json"),
                ("derived_deck_seat0", "seat0-deck.txt"),
                ("derived_deck_seat1", "seat1-deck.txt"),
                ("executed_provenance_seat0", "input-provenance-seat0.json"),
                ("executed_provenance_seat1", "input-provenance-seat1.json"),
            ] {
                artifacts.insert(role.into(), attempt.join(name));
            }
            for seat in 0..2 {
                artifacts.insert(
                    format!("transcript_seat{seat}"),
                    attempt.join(format!("transcripts/{game}-seat{seat}.jsonl")),
                );
            }
            for path in artifacts.values() {
                ensure!(
                    path.is_file() && path.canonicalize()?.starts_with(attempt),
                    "Invalid original unbound artifact"
                );
            }
            return Ok(Self {
                game,
                technical,
                request,
                artifacts,
            });
        }
        ensure!(
            accepted["status"] == "admitted"
                && accepted["technical"] == technical
                && technical["status"] == "complete"
                && technical["game_id"] == game,
            "Invalid original technical admission"
        );
        for artifact in technical["artifacts"]
            .as_array()
            .context("Missing collector artifacts")?
        {
            let path = Path::new(artifact["path"].as_str().context("Missing artifact path")?)
                .canonicalize()?;
            let role = artifact["role"].as_str().context("Missing artifact role")?;
            ensure!(
                path.starts_with(attempt)
                    && artifact["sha256"] == hash(&read(&path)?)
                    && artifacts.insert(role.into(), path).is_none(),
                "Changed, invalid or repeated artifact role {role}"
            );
        }
        Ok(Self {
            game,
            technical,
            request,
            artifacts,
        })
    }

    pub(super) fn artifact(&self, role: &str) -> Result<&Path> {
        Ok(self
            .artifacts
            .get(role)
            .with_context(|| format!("Missing artifact {role}"))?)
    }
}

struct PublicTape {
    views: BTreeMap<Option<u64>, Value>,
    latest: Value,
    checked: usize,
    receipt: Value,
    sha256: String,
}

impl PublicTape {
    fn load(receipt: &Receipt, build: &str) -> Result<Self> {
        let bytes = read(receipt.artifact("replay_tape")?)?;
        let tape: Vec<Value> = lines(&bytes)?;
        let sha256 = hash(&bytes);
        let header = tape.first().context("No public tape header")?;
        let terminal = tape.last().context("No public tape end")?;
        ensure!(
            header["game"] == receipt.game
                && header["perspective"] == "public"
                && header["build"]
                    .as_str()
                    .is_some_and(|stamp| stamp.contains(&build[..10]))
                && terminal["kind"] == "stop"
                && terminal["stop"] == "end"
                && terminal["diverged"].is_null(),
            "Tape identity or exact-end mismatch"
        );
        let proof = if receipt.technical["status"] == "complete" {
            let proof = json_file(receipt.artifact("replay_receipt")?)?;
            ensure!(
                terminal == &proof["stop"]
                    && proof["gateway_info"]["commit"] == build
                    && proof["status"] == "verified"
                    && proof["record_sha256"] == receipt.technical["record_sha256"]
                    && proof["tape_sha256"] == sha256,
                "Original tape receipt mismatch"
            );
            proof
        } else {
            json!({"stop":terminal,"header":header})
        };
        let mut views = BTreeMap::new();
        for view in tape.into_iter().filter(|row| row["kind"] == "view") {
            ensure!(
                view["check"] == "exact" && views.insert(view["n"].as_u64(), view).is_none(),
                "Duplicate or unverified public frame"
            );
        }
        Ok(Self {
            views,
            latest: Value::Null,
            checked: 0,
            receipt: proof,
            sha256,
        })
    }

    fn compare_step(&mut self, n: Option<u64>, session: &StandardSession) -> Result<()> {
        let projected = json!(session.spectator_view().into_player_view(PlayerId::new(0)));
        if let Some(frame) = self.views.get(&n) {
            ensure!(
                projected == frame["json"],
                "Public view differs after {n:?}"
            );
            self.latest = frame.clone();
            self.checked += 1;
        } else {
            ensure!(
                without_seq(projected) == without_seq(self.latest["json"].clone()),
                "Published tape omits changed public state after {n:?}"
            );
        }
        Ok(())
    }

    fn verify_native_projector(&self, record: &[u8]) -> Result<()> {
        use baylee_gamehost::replay::{self, Check, Perspective, Stop, Verify};
        use baylee_protocol::v1::envelope::Msg;
        use std::ops::ControlFlow;
        let mut observed = BTreeSet::new();
        let mut exact = true;
        let projection = replay::project(
            record,
            vec!["Seat 0".into(), "Seat 1".into()],
            Perspective::Public,
            Verify::Stop,
            |moment| {
                exact &= moment.check == Check::Exact;
                for envelope in moment.frames {
                    if let Some(Msg::StateDelta(delta)) = envelope.msg {
                        let native = baylee_view::ViewFrame::whole(&delta.view_frame);
                        exact &= self.views.get(&moment.input).is_some_and(|frame| {
                            native.is_some_and(|native| frame["json"] == json!(native))
                        });
                        exact &= observed.insert(moment.input);
                    }
                }
                ControlFlow::Continue(())
            },
        )
        .map_err(|error| anyhow::anyhow!("Native public projection failed: {error:?}"))?;
        ensure!(
            exact
                && projection.stop == Stop::End
                && projection.diverged.is_none()
                && json!(projection.inputs) == self.receipt["stop"]["inputs"]
                && json!(projection.exact_to) == self.receipt["stop"]["exact_to"]
                && observed.len() == self.views.len(),
            "Native public projection disagrees with the original public tape"
        );
        Ok(())
    }
}

pub(super) fn without_seq(mut json: Value) -> Value {
    if let Some(object) = json.as_object_mut() {
        object.remove("seq");
    }
    json
}

#[allow(
    clippy::too_many_lines,
    reason = "One exact replay beside the public tape and both seats"
)]
pub(super) fn audit(attempt: &Path, approved_decks: &Path, pins: &Pins<'_>) -> Result<Value> {
    let receipt = Receipt::load(attempt)?;
    let record_bytes = read(receipt.artifact("anonymized_record")?)?;
    let record_sha256 = hash(&record_bytes);
    ensure!(
        receipt.technical["record_sha256"].is_null()
            || receipt.technical["record_sha256"] == record_sha256
    );
    let record: Vec<Line> = lines(&record_bytes)?;
    let Some(Line::Header {
        preset,
        hash: initial_hash,
        build,
        ..
    }) = record.first()
    else {
        anyhow::bail!("No native record header");
    };
    ensure!(
        build.contains(&pins.build[..10]),
        "Record is from a different build"
    );
    let contexts = contexts(&receipt, preset, approved_decks)?;
    let bindings = json_file(receipt.artifact("input_receipt")?)?;
    ensure!(
        bindings["requested_models"] == receipt.request["models"]
            && bindings["requested_minds"]
                .as_array()
                .is_some_and(|minds| minds.len() == 2)
            && bindings["requested_room_setup"] == receipt.request["room_setup"]
            && bindings["requested_fallbacks"] == json!([[], []]),
        "Collector configuration binding differs"
    );
    for (path, hash_value) in bindings["sha256"]
        .as_object()
        .context("Missing input hashes")?
    {
        ensure!(
            hash(&read(Path::new(path))?) == *hash_value,
            "Original collector input changed"
        );
    }
    let configured = json_file(receipt.artifact("room_setup_receipt")?)?;
    ensure!(
        configured["game_id"] == receipt.game
            && configured["confirmed_setup"] == bindings["requested_room_setup"]
            && configured["confirmed_setup"]["starting_life"] == 20
            && configured["confirmed_setup"]["free_mulligans"] == 0
            && preset
                .seats
                .iter()
                .all(|seat| seat.starting_life == Some(20)
                    && seat.starting_hand.is_none()
                    && seat.starting_battlefield.is_empty())
            && preset.house_rules.free_mulligans == Some(0),
        "Game setup differs from the TRAIN stamp"
    );
    let mut seats = [
        Seat::load(attempt, &receipt, 0, &record, &contexts[0])?,
        Seat::load(attempt, &receipt, 1, &record, &contexts[1])?,
    ];
    let mut session =
        StandardSession::with_detection(preset).context("Cannot build native standard session")?;
    ensure!(
        &format!("{:016x}", session.snapshot_hash()) == initial_hash,
        "Initial engine hash differs"
    );
    let mut tape = PublicTape::load(&receipt, pins.build)?;
    tape.verify_native_projector(&unpack(&record_bytes)?)?;
    let mut public_objects = Objects::default();
    let mut known_objects = [Objects::default(), Objects::default()];
    tape.compare_step(None, &session)?;
    visit_objects(&session, &mut public_objects, &mut known_objects);
    for seat in &mut seats {
        seat.capture(&mut session, &tape.latest)?;
    }
    let mut declarations = [0, 0];
    let mut inputs = 0;
    let mut last_n = None;
    let mut terminal = None;
    for entry in &record[1..] {
        match entry {
            Line::DeclaredMind { seat, mind, .. } => {
                let s = usize::from(*seat);
                ensure!(
                    s < seats.len()
                        && mind.kind == MindKind::LlmCli
                        && mind.provider == "codex"
                        && receipt.request["models"][s] == mind.model.as_str()
                        && mind.effort == "low"
                        && json!(mind) == bindings["requested_minds"][s],
                    "Unexpected active mind on seat {seat}"
                );
                declarations[s] += 1;
            }
            Line::Input {
                n,
                at,
                seat,
                by,
                action,
                hash,
                ..
            } => {
                let s = usize::from(*seat);
                ensure!(
                    s < seats.len()
                        && declarations[s] > 0
                        && last_n.is_none_or(|old| *n > old)
                        && terminal.is_none(),
                    "Invalid record input sequence"
                );
                ensure!(
                    *by == Source::Seat,
                    "TRAIN duel contains a non-socket decision at {n}"
                );
                seats[s].verify_input(&mut session, *n, action)?;
                session.tell_time(*at);
                session
                    .act_slice(PlayerId::new(*seat), action.clone(), 0)
                    .map_err(|reason| {
                        anyhow::anyhow!("Native seat action refused input {n}: {reason}")
                    })?;
                ensure!(
                    &format!("{:016x}", session.snapshot_hash()) == hash,
                    "Engine hash differs after {n}"
                );
                tape.compare_step(Some(*n), &session)?;
                visit_objects(&session, &mut public_objects, &mut known_objects);
                for seat in &mut seats {
                    seat.capture(&mut session, &tape.latest)?;
                }
                inputs += 1;
                last_n = Some(*n);
            }
            Line::End {
                n, turn, reason, ..
            } => {
                ensure!(
                    matches!(session.pending(), Pending::GameOver(_)) && terminal.is_none(),
                    "Record end lacks the engine's natural terminal"
                );
                terminal = Some(json!({"n":n, "turn":turn, "reason":reason}));
            }
            Line::Chair { .. } => {}
            Line::Header { .. } => anyhow::bail!("Second record header"),
        }
    }
    ensure!(
        matches!(record.last(), Some(Line::End { .. }))
            && declarations.iter().all(|count| *count > 0)
            && inputs > 0
            && tape.checked == tape.views.len()
            && tape.receipt["stop"]["inputs"] == inputs
            && tape.receipt["stop"]["exact_to"] == json!(last_n),
        "Replay is incomplete"
    );
    let [a, b] = seats;
    let mut report = json!({
        "run_kind":"fresh_TRAIN_only",
        "game_id":receipt.game, "attempt_directory":attempt, "engine_commit":pins.build,
        "decks":receipt.request["decks"].as_array()
            .context("Missing deck keys")?.iter().map(|deck| &deck["key"]).collect::<Vec<_>>(),
        "approved_decks_sha256":hash(&read(approved_decks)?), "train_manifest_sha256":pins.manifest_sha256,
        "record_artifact_sha256":record_sha256,"public_tape_sha256":tape.sha256,
        "record_inputs":inputs, "full_engine_hashes_exact":true, "public_views":tape.checked,
        "terminal":terminal, "seats":[a.report()?, b.report()?], "ppo_eligible":false, "weight_updates":0,
        "deck_card_ids":contexts.each_ref().map(|context| deck_counts(&context.deck).into_keys().collect::<Vec<_>>()),
        "public_objects":public_objects.report(),
        "seat_visible_objects":known_objects.each_ref().map(Objects::report),
        "public_tape_stop":tape.receipt["stop"],
        "native_public_projector_verified":true,
        "initial_engine_hash":initial_hash,"terminal_engine_hash":format!("{:016x}",session.snapshot_hash()),
    });
    if receipt.request["models"] == json!([ASTRA, ASTRA]) {
        report["schema_version"] = json!(1);
        report["audit_contract"] = json!("astra_direct_executed_v1");
        report["model"] = json!(format!("codex:{ASTRA}"));
        report["effort"] = json!("low");
    } else {
        report["schema_version"] = json!(2);
        report["audit_contract"] = json!("codex_direct_executed_v2");
        for seat in 0..2 {
            report["seats"][seat]["mind"] = bindings["requested_minds"][seat].clone();
        }
    }
    Ok(report)
}

fn visit_objects(session: &StandardSession, public: &mut Objects, seats: &mut [Objects; 2]) {
    public.observe(&session.spectator_view().into_player_view(PlayerId::new(0)));
    for (seat, objects) in seats.iter_mut().enumerate() {
        objects
            .observe(&session.observe_view(PlayerId::new(u8::try_from(seat).expect("duel seat"))));
    }
}

fn contexts(
    receipt: &Receipt,
    preset: &GamePreset,
    approved_decks: &Path,
) -> Result<[Arc<GameContext>; 2]> {
    let approved = json_file(approved_decks)?;
    let rows = approved["decks"]
        .as_array()
        .context("Missing approved decks")?;
    ensure!(
        preset.seats.len() == 2 && rows.len() == 2,
        "Invalid duel deck count"
    );
    let mut checked = BTreeSet::new();
    let mut contexts = Vec::new();
    for seat in 0..2 {
        let key = receipt.request["decks"][seat]["key"]
            .as_str()
            .context("Missing request deck key")?;
        let approved = rows
            .iter()
            .find(|row| row["key"] == key)
            .context("Deck outside approved pairing")?;
        ensure!(
            checked.insert(key) && approved["split"] == "train",
            "Invalid approved deck split"
        );
        let expected = Path::new(
            approved["path"]
                .as_str()
                .context("Missing approved deck path")?,
        );
        let dealt = Deck::from_file(receipt.artifact(&format!("derived_deck_seat{seat}"))?)
            .with_context(|| format!("Reading bridge deck for seat {seat}"))?;
        let expected = approved_counts(expected)?;
        let engine_deck = &preset.seats[seat];
        ensure!(
            deck_counts(&dealt.list) == expected
                && engine_counts(engine_deck.deck.iter().map(|entry| entry.card))
                    == deck_counts(&dealt.list)
                && dealt.list.commanders.is_empty()
                && engine_deck.commanders.is_empty(),
            "The requested, uploaded, and engine main decks differ for seat {seat}"
        );
        contexts.push(Arc::new(GameContext {
            game_id: receipt.game.clone(),
            seat: PlayerId::new(seat as u8),
            seats: 2,
            teams: preset.seats.iter().map(|seat| seat.team).collect(),
            format: preset.format,
            deck: dealt.list,
            decision_secs: None,
        }));
    }
    contexts
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid deck context count"))
}

fn approved_counts(path: &Path) -> Result<BTreeMap<CardIndex, u32>> {
    use baylee_core::acceptance::{Zone, parse_decks};
    let mut counts = BTreeMap::new();
    for row in parse_decks(std::str::from_utf8(&read(path)?)?)? {
        if row.zone == Zone::Main {
            let card =
                baylee_cards::decks::by_name(&row.name).context("Uncompiled approved card")?;
            *counts.entry(card).or_default() += row.count;
        }
    }
    Ok(counts)
}

fn engine_counts(entries: impl IntoIterator<Item = CardIndex>) -> BTreeMap<CardIndex, u32> {
    let mut counts = BTreeMap::new();
    for entry in entries {
        *counts.entry(entry).or_default() += 1;
    }
    counts
}

fn deck_counts(deck: &DeckList) -> BTreeMap<CardIndex, u32> {
    let mut counts = BTreeMap::new();
    for entry in &deck.main {
        *counts.entry(entry.card).or_default() += entry.count;
    }
    counts
}
