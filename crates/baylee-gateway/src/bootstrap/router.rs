//! Existing route/middleware order, extracted without behavior changes.
use super::{
    Router, Shared, account_routes, archive_replay, art, banlists, catalog_search, catalog_text,
    central, chair, challenges, cors, cosmetics, create_game, deck_routes, departure, engine,
    game_cosmetics, game_ws, get, hand_over, health, info, join_game, leave_game, list_games,
    list_replays, list_seats, lobby_stats, lobby_ws, metrics, pool, poolfacts, post, put, rematch,
    replay_tape, report, room, routes, seathost, set_ready, set_seat, share_replay, social, source,
    start_room, take_seat, tower, watch_ws, whole_body, ws_ticket,
};

pub(super) fn build(state: &Shared) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/source", get(source))
        .route("/info", get(info))
        .merge(account_routes())
        .merge(deck_routes())
        .merge(banlists::routes())
        .merge(report::routes())
        .merge(report::direct::routes())
        .merge(social::routes::routes())
        .route("/social/ws", get(social::socket::social_ws))
        .route("/lobby/games", get(list_games).post(create_game))
        .route("/challenges/{attempt}/play", post(challenges::play))
        .route("/tower", get(tower::routes::available))
        .route("/tower/{run}/play", post(tower::routes::play))
        .route("/lobby/seats", get(list_seats))
        .route("/lobby/stats", get(lobby_stats))
        // Replays (`docs/protocol.md` §"Replays"): the games an account sat
        // at, and one of them played again as a tape.
        .route("/replays", get(list_replays))
        .route("/replays/{game}/tape", post(replay_tape))
        .route("/replays/{game}/share", put(share_replay))
        .route("/replays/{game}/archived", put(archive_replay))
        .route("/lobby/depart", post(departure::depart))
        .route("/lobby/games/{id}/join", post(join_game))
        .route("/lobby/games/{id}/configure", post(room::configure))
        .route(
            "/lobby/games/{id}/list",
            get(routes::room_list::read).post(routes::room_list::choose),
        )
        .route("/lobby/games/{id}/seat", post(take_seat))
        .route("/lobby/games/{id}/seats/{seat}", post(set_seat))
        .route("/lobby/games/{id}/ready", post(set_ready))
        .route("/lobby/games/{id}/start", post(start_room))
        .route("/lobby/games/{id}/host", post(hand_over))
        .route("/lobby/games/{id}/leave", post(leave_game))
        .route("/lobby/games/{id}/rematch", post(rematch))
        .route("/lobby/games/{id}/chairs/{seat}/ticket", post(chair::mint))
        .route(
            "/lobby/games/{id}/chairs/{seat}/redeem",
            post(chair::redeem),
        )
        .route(
            "/lobby/games/{id}/chairs/{seat}/hosted",
            post(seathost::order).delete(seathost::cancel),
        )
        .route("/lobby/llm-profiles", get(seathost::profiles))
        .route("/seathost/ws", get(seathost::socket))
        .route("/lobby/games/{id}/chair", get(chair::status))
        .route("/lobby/games/{id}/chair/leave", post(chair::leave))
        .route("/lobby/games/{id}/chair/ready", post(chair::ready))
        .route("/ws-ticket", post(ws_ticket))
        .route("/lobby/ws", get(lobby_ws))
        .route("/games/{id}/ws", get(game_ws))
        .route("/games/{id}/watch", get(watch_ws))
        .route("/games/{id}/cosmetics", get(game_cosmetics))
        // The control and engine planes. Neither carries a player's traffic
        // and neither accepts a player's token; see `engine.rs`.
        .route("/agent/ws", get(engine::agent_ws))
        .route("/engine/ws", get(engine::engine_ws))
        .route("/pool", get(pool::pool))
        .route("/pool/facts", get(poolfacts::pool_facts))
        .route("/printings", get(pool::printings))
        .route("/catalog/text", get(catalog_text))
        .route("/catalog/search", get(catalog_search))
        // Card art, mirrored from Scryfall by printing id for this gateway's
        // own players: a session is required, and it is an id cache rather
        // than a proxy — see `art.rs`.
        .route("/art/{size}/{face}/{a}/{b}/{file}", get(art::art))
        // Axum caps a body at 2 MB by default, which is under a phone
        // photograph. The cap that matters is the one in `cosmetics`,
        // checked again before anything is decoded.
        // The kind rides in the query rather than the path, and not by
        // preference: axum registers a route by its path before it looks at
        // the method, so `/images/{kind}` and `/images/{file}` are the same
        // route wearing two different parameter names, and building the
        // router panics. It panicked at startup, which meant the gateway
        // never served and every end-to-end test in the crate failed
        // with "connection refused" — a message that says nothing about
        // routing. `/images` is the collection and `/images/{file}` is one
        // member of it, which is the shape that had no conflict to resolve.
        .route(
            "/images",
            post(cosmetics::upload).layer(axum::extract::DefaultBodyLimit::max(
                cosmetics::MAX_UPLOAD_BYTES,
            )),
        )
        .route("/images/{id}", get(cosmetics::serve))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            central::barrier,
        ))
        .merge(central::router())
        // Outside the barrier: a body still on its way holds nothing.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            whole_body,
        ))
        // Inside the CORS layer, so a preflight is not a request served.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            metrics::count,
        ))
        .layer(axum::middleware::from_fn(cors))
        .with_state(state.clone())
}
