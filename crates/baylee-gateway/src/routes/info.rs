//! What the gateway says about itself: its source (AGPL §13), its build,
//! its health, and how it signs people in.

use crate::{Json, Shared, State, StatusCode, clock};

/// What this gateway is, and where the source for exactly this build lives.
///
/// The AGPL's §13 obliges a program modified and offered to users over a
/// network to offer those users its Corresponding Source, and a gateway is
/// the one process here that meets that description. Unauthenticated on
/// purpose: an offer conditional on having an account is not an offer to the
/// people §13 is about. It names the commit rather than only the repository,
/// because "the source is on GitHub" does not say *which* source — a build
/// running a patch nobody published would answer that sentence truthfully
/// and still be hiding what it runs.
pub(crate) async fn source(State(state): State<Shared>) -> Json<serde_json::Value> {
    let mut body = build_fields();
    body.insert("name".into(), "baylee".into());
    body.insert("license".into(), "AGPL-3.0-only".into());
    body.insert("source".into(), state.source_url.clone().into());
    Json(body.into())
}

/// The longest `BAYLEE_SOURCE_URL` may be, in characters.
pub(crate) const MAX_SOURCE_CHARS: usize = 200;

/// Where this gateway's source can be had, out of `BAYLEE_SOURCE_URL`.
///
/// §13 of the AGPL makes whoever runs a *modified* gateway offer its users
/// that version's source, and a fork may publish it somewhere other than
/// the repository its `Cargo.toml` names. So an operator can say where;
/// unset or blank is the repository this build was made from
/// (`baylee_build::REPOSITORY`). It is shown to players as it is (the
/// client's front door draws it), so it has to be a web address and nothing
/// that displays as something else: `http://` or `https://`, no whitespace,
/// no control or bidirectional character, at most [`MAX_SOURCE_CHARS`]. Refused
/// at startup rather than trimmed, as `BAYLEE_GATEWAY_NAME` is.
pub(crate) fn source_url(raw: Option<&str>) -> Result<String, String> {
    let Some(url) = raw.map(str::trim).filter(|url| !url.is_empty()) else {
        return Ok(baylee_build::REPOSITORY.to_owned());
    };
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("it is not an http:// or https:// address".to_owned());
    }
    let length = url.chars().count();
    if length > MAX_SOURCE_CHARS {
        return Err(format!(
            "{length} characters, and an address has {MAX_SOURCE_CHARS} at most"
        ));
    }
    if let Some(bad) = url
        .chars()
        .find(|&c| c.is_whitespace() || c.is_control() || is_bidi_control(c))
    {
        return Err(format!("it contains U+{:04X}", u32::from(bad)));
    }
    Ok(url.to_owned())
}

/// Which binary this is, spelled once for every route that says so.
///
/// `/source`, `/health` and `/info` all answer it, and all three build it
/// here from the same `baylee_build` constants, so "I rebuilt it" is
/// checkable against any of them and no two can drift into disagreeing about
/// one binary.
pub(crate) fn build_fields() -> serde_json::Map<String, serde_json::Value> {
    let mut fields = serde_json::Map::new();
    fields.insert("version".into(), baylee_build::short().into());
    fields.insert("commit".into(), baylee_build::COMMIT.into());
    fields.insert("build".into(), baylee_build::BUILD_NUMBER.into());
    fields.insert("built_at".into(), baylee_build::BUILT_AT.into());
    // Stated rather than implied: a reader who finds `dirty` true knows the
    // commit above does not fully describe what is running, which is the one
    // case where the §13 offer would otherwise mislead.
    fields.insert("dirty".into(), baylee_build::DIRTY.into());
    fields
}

/// What a client asks before it saves this gateway: the name to show, the
/// build, the two versions that decide whether the two can talk, and where
/// the source is (`source`, the AGPL's §13 offer, which the client's front
/// door draws; see [`source_url`]).
///
/// Unauthenticated, because it is asked before there is an account, and it
/// carries nothing a stranger may not read. `protocol_version` is the
/// envelope a seat socket speaks. `view_version` is the view shape *this
/// gateway's build* was compiled with, and it is a promise about nothing
/// else: the gateway never decodes a view, and an agent's engine says only
/// its protocol when it attaches (#271). A deployment runs one build on both sides, so
/// the number is the right early warning; the check that decides is still
/// the client's own, on the first `GameStatic` of a game.
pub(crate) async fn info(State(state): State<Shared>) -> Json<serde_json::Value> {
    let mut body = build_fields();
    if let Some(name) = &state.display_name {
        body.insert("name".into(), name.clone().into());
    }
    body.insert(
        "protocol_version".into(),
        baylee_protocol::PROTOCOL_VERSION.into(),
    );
    body.insert("view_version".into(), baylee_view::VIEW_VERSION.into());
    body.insert("source".into(), state.source_url.clone().into());
    // Who may come in (#317): `open`, `invite` (a closed-beta key for a new
    // account or guest) or `off`, and whether guests are taken at all.
    body.insert("registration".into(), state.registration.wire().into());
    body.insert("guests".into(), state.guests_enabled.into());
    // Whether a host may seat a model this gateway's seat agents run
    // (`docs/protocol.md` §"Hosted language-model seats").
    body.insert(
        "hosted_llm".into(),
        (state.seathost_token.is_some() && !state.seathosts.lock().hosts.is_empty()).into(),
    );
    // The version of the terms a player accepts here (WG-1), `null` when
    // the gateway has none; `GET /terms` has the text.
    body.insert(
        "terms".into(),
        state
            .terms
            .as_ref()
            .map_or(serde_json::Value::Null, |terms| {
                terms.version.clone().into()
            }),
    );
    Json(body.into())
}

/// The longest name `BAYLEE_GATEWAY_NAME` may set, in characters.
///
/// The client caps what it shows at the same length on its own
/// (`baylee_client_core::lobby::gateway_info::MAX_NAME_CHARS`), because a gateway
/// is a stranger to it and this check only binds gateways built from here.
pub(crate) const MAX_NAME_CHARS: usize = 64;

/// The name a gateway shows a client, out of `BAYLEE_GATEWAY_NAME`.
///
/// Unset or blank is no name, and the client shows the address. A name that
/// is too long, or that carries a control character or a bidirectional
/// override, is refused rather than trimmed. The override can make a line
/// display differently from what it says, and the two were written as one
/// string by somebody, so the operator is told at startup rather than every
/// player seeing a quietly altered one.
pub(crate) fn display_name(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(name) = raw.map(str::trim).filter(|name| !name.is_empty()) else {
        return Ok(None);
    };
    let length = name.chars().count();
    if length > MAX_NAME_CHARS {
        return Err(format!(
            "{length} characters, and a name has {MAX_NAME_CHARS} at most"
        ));
    }
    if let Some(bad) = name.chars().find(|&c| c.is_control() || is_bidi_control(c)) {
        return Err(format!("it contains U+{:04X}", u32::from(bad)));
    }
    Ok(Some(name.to_owned()))
}

/// The characters that reorder how a line of text is displayed (Unicode's
/// embeddings, overrides and isolates) without being shown themselves.
///
/// The client drops the same set from whatever a gateway sends it (its own
/// `is_bidi_control` in `baylee_client_core::lobby::gateway_info`); the two
/// lists are one list.
pub(crate) fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// How long any one probe in `/health` may take.
///
/// Bounded, because a health route that hangs is strictly worse than one that
/// answers "down": a monitor blocked on a socket reports nothing at all, and
/// "nothing" is indistinguishable from "not scraped yet". Two seconds is long
/// enough that a loaded but working database still answers — the e2e suite
/// runs three dozen gateways against one server at a pool of two apiece — and
/// short enough that the caller gets a verdict rather than a timeout of its
/// own.
pub(crate) const HEALTH_PROBE: std::time::Duration = std::time::Duration::from_secs(2);

/// `GET /health` — whether this gateway can do its job, and what it cannot.
///
/// The route exists because the only evidence this process was alive used to
/// be a line in its log and an open port, and an open port only says that
/// `bind` succeeded. That is a weaker claim than it looks from *both* sides:
/// it says nothing about the states below, and — since binding is the last
/// thing `main` does — it also cannot be observed until everything else has
/// already worked.
///
/// Unauthenticated, because a monitor that needs a token is a monitor nobody
/// wires up, and because there is nothing here to protect. No account name,
/// no token, no store path, no configured URL, no counts that are not already
/// visible in the lobby listing: every field is a bit or a number about this
/// process, and the version is one `/source` already serves to anyone.
///
/// # What the status code carries
///
/// Exactly one question — is the database there — because that is the only
/// state this process cannot work around. `DATABASE_URL` is required: a
/// gateway whose database has gone away keeps its port open and its log
/// quiet while answering every route that matters with a 503, which is the
/// precise failure this route was asked for.
///
/// Everything else is a field and never a code. A gateway with no agent
/// connected hosts no games, and it is still a legitimate thing to be
/// running — the e2e suite spawns three dozen agentless ones — so
/// `agents.connected: 0` is reported rather than escalated. The same goes for
/// a catalog that was never ingested: no card text is a thinner client, not a
/// broken gateway.
pub(crate) async fn health(State(state): State<Shared>) -> (StatusCode, Json<serde_json::Value>) {
    let database = matches!(
        tokio::time::timeout(HEALTH_PROBE, state.db.ping()).await,
        Ok(Ok(()))
    );

    // Four states rather than a bit, because they want four different
    // answers from whoever is reading. `off` is a choice, `empty` wants an
    // ingest, `projection_missing` wants `baylee-catalog project` and is the
    // one that answers every search with nothing while erroring at nobody,
    // and `unreachable` is the database being gone — already in the code
    // above, repeated here so one field is not read as covering for another.
    let catalog = match state.catalog.as_ref() {
        None => serde_json::json!({ "state": "off" }),
        Some(catalog) => match tokio::time::timeout(HEALTH_PROBE, catalog.readiness()).await {
            Ok(Ok(found)) => serde_json::json!({
                "state": match (found.cards, found.projection) {
                    (true, true) => "ready",
                    (true, false) => "projection_missing",
                    _ => "empty",
                },
                "cards": found.cards,
                "projection": found.projection,
            }),
            _ => serde_json::json!({ "state": "unreachable" }),
        },
    };

    let (agents_connected, agent_games) = {
        let agents = state.agents.lock();
        (
            agents.connected.len(),
            agents
                .connected
                .values()
                .map(|agent| agent.games.len())
                .sum::<usize>(),
        )
    };

    // Counted from the lobby rather than from the agents, because the two
    // disagree in the one case worth seeing: a game the gateway has ordered
    // but whose engine has not dialled back yet is on an agent's list and has
    // no `EngineLink`. That gap is what `seats_awaiting_engine` is.
    let (games_running, games_local, games_waiting, seats_awaiting_engine) = {
        let lobby = state.lobby.lock();
        let playing = lobby.running();
        (
            playing.clone().count(),
            // Ordered from an agent on this machine's unix socket. What a
            // deploy of this machine waits for: a game on an agent elsewhere
            // is not ended by replacing this machine's engine binary.
            playing.clone().filter(|game| game.engine_local).count(),
            lobby.waiting().count(),
            playing
                .filter(|game| game.engine.is_none())
                .map(|game| game.seats.len())
                .sum::<usize>(),
        )
    };

    let code = if database {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let mut body = build_fields();
    body.insert("ok".into(), database.into());
    body.insert("database".into(), database.into());
    body.insert("catalog".into(), catalog);
    body.insert(
        "agents".into(),
        serde_json::json!({
            "connected": agents_connected,
            "games": agent_games,
        }),
    );
    // Seat agents (`seathost.rs`): how many, and the bridges they run.
    let (seathosts, hosted_games) = {
        let registry = state.seathosts.lock();
        (
            registry.hosts.len(),
            registry
                .hosts
                .values()
                .flat_map(|host| &host.profiles)
                .map(|profile| profile.games)
                .sum::<u32>(),
        )
    };
    body.insert(
        "seathosts".into(),
        serde_json::json!({ "connected": seathosts, "games": hosted_games }),
    );
    // Whether a new game may start (`admission.rs`): what a deploy that
    // drains every agent's games checks before it relies on the hold.
    body.insert(
        "admission".into(),
        state.admission.admission().wire().into(),
    );
    body.insert(
        "games".into(),
        serde_json::json!({
            "running": games_running,
            "local_running": games_local,
            "waiting": games_waiting,
            "seats_awaiting_engine": seats_awaiting_engine,
        }),
    );
    (code, Json(body.into()))
}

/// Public auth configuration (clients check this before offering
/// registration).
pub(crate) async fn auth_config(State(state): State<Shared>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        // True for `invite` too: a client from before keys offers the form,
        // and the refusal it gets tells it to update (#317).
        "registration_enabled": state.registration.takes_sign_ups(),
        // `open`, `invite` or `off` (#317); a client that knows keys reads
        // this one.
        "registration": state.registration.wire(),
        // Whether "play as a guest" is offered (#269).
        "guests_enabled": state.guests_enabled,
        // Whether `GET /art/…` mirrors card images. A client that pointed at a
        // gateway with the mirror switched off would get a 404 for every card
        // and draw a whole table of constructed faces, so it is told here
        // rather than discovering it one blank card at a time.
        "art_cache": state.art.enabled(),
        "deck_images": state.deck_images.enabled(),
        // The clocks a room may be opened at, so a client builds its picker
        // from what this gateway actually accepts instead of hard-coding a
        // list that goes stale the day one is added. The first is the
        // default, which is what a room gets by saying nothing.
        "clocks": clock::PRESETS.iter().map(|preset| serde_json::json!({
            "name": preset.name,
            "decide_secs": preset.decision_timeout_secs,
            "reconnect_secs": state.reconnect_secs,
            "blurb": preset.blurb,
        })).collect::<Vec<_>>(),
    }))
}
