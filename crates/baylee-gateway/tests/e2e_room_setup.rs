//! Room configuration goes through real HTTP, authority checks, and engine setup.
mod common;
use common::{attach_agent_watching, http, json_field, login, spawn_gateway};
use serde_json::{Value, json};

#[allow(clippy::needless_pass_by_value)] // accepts temporary JSON request bodies
fn post(port: u16, token: &str, path: &str, body: Value) -> (u16, String) {
    http(port, "POST", path, Some(token), &body.to_string())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[allow(clippy::too_many_lines)] // one room's complete lifecycle
async fn configuration_is_host_owned_and_reaches_the_engine() {
    let gateway = spawn_gateway("room-setup");
    let port = gateway.port;
    let (agent, mut presets) = attach_agent_watching(&gateway).await;
    let host = login(port, "roomhost", "Host");
    let first = login(port, "firstguest", "First");
    let second = login(port, "secondguest", "Second");
    let (status, body) = post(
        port,
        &host,
        "/lobby/games",
        json!({"mode":"open","seats":4}),
    );
    assert_eq!(status, 200, "no deck needed: {body}");
    let id = json_field(&body, "game_id").to_string();
    let base = format!("/lobby/games/{id}");
    let settings = json!({"name":"Moon garden", "chairs":4, "password":"moonlight", "setup":{
        "starting_life":30, "free_mulligans":3,
        "seats":[{"life":45,"permanents":["Forest *F*","Island"],"counters":[[{"kind":"charge","amount":3}]]}]
    }});
    assert_eq!(
        post(port, &first, &format!("{base}/configure"), settings.clone()).0,
        403
    );
    assert_eq!(
        post(port, &host, &format!("{base}/configure"), settings.clone()).0,
        200
    );
    assert_eq!(
        post(port, &first, &format!("{base}/join"), json!({})).0,
        403
    );
    assert_eq!(
        post(
            port,
            &first,
            &format!("{base}/join"),
            json!({"seat":3,"password":"moonlight"})
        )
        .0,
        200
    );
    assert_eq!(
        post(
            port,
            &second,
            &format!("{base}/join"),
            json!({"seat":1,"password":"moonlight"})
        )
        .0,
        200
    );
    assert_eq!(
        post(port, &first, &format!("{base}/ready"), json!({})).0,
        409
    );
    let mut smaller = settings.clone();
    smaller["chairs"] = json!(2);
    assert_eq!(
        post(port, &host, &format!("{base}/configure"), smaller).0,
        409
    );
    let mut invalid = settings.clone();
    invalid["setup"]["seats"][0]["permanents"] = json!(["Counterspell"]);
    assert_eq!(
        post(port, &host, &format!("{base}/configure"), invalid).0,
        400
    );
    // Invalid AI profile must not partially convert the open chair.
    assert_eq!(
        post(
            port,
            &host,
            &format!("{base}/seats/2"),
            json!({"kind":"ai","ai":"invented"})
        )
        .0,
        400
    );
    let (_, listing) = http(
        port,
        "GET",
        &format!("/lobby/games?q={id}"),
        Some(&first),
        "",
    );
    let listing: Value = serde_json::from_str(&listing).unwrap();
    let room = &listing["games"][0];
    assert_eq!(
        room["setup"]["seats"][0]["permanents"],
        json!(["Forest *F*", "Island"])
    );
    assert_eq!(room["seats"][2]["kind"], "human");
    assert!(!listing.to_string().contains("moonlight"));
    // Host succession follows arrival time, not chair index.
    assert_eq!(
        post(port, &host, &format!("{base}/leave"), json!({})).0,
        204
    );
    let mut settings = settings;
    settings["password"] = Value::Null;
    assert_eq!(
        post(
            port,
            &second,
            &format!("{base}/configure"),
            settings.clone()
        )
        .0,
        403
    );
    assert_eq!(
        post(port, &first, &format!("{base}/configure"), settings.clone()).0,
        200
    );
    for seat in [0, 2] {
        assert_eq!(
            post(
                port,
                &first,
                &format!("{base}/seats/{seat}"),
                json!({"kind":"ai","ai":"sharp"})
            )
            .0,
            200
        );
    }
    for (token, seat) in [(&first, 3), (&second, 1)] {
        let (status, deck) = post(
            port,
            token,
            "/decks",
            json!({"name":"Woodland", "cards":["40 Forest","20 Swamp"]}),
        );
        assert_eq!(status, 200, "{deck}");
        let deck_id = json_field(&deck, "deck_id");
        assert_eq!(
            post(
                port,
                token,
                &format!("{base}/seats/{seat}"),
                json!({"deck_id":deck_id})
            )
            .0,
            200
        );
        assert_eq!(
            post(port, token, &format!("{base}/ready"), json!({})).0,
            200
        );
    }
    settings["setup"]["free_mulligans"] = json!(2);
    assert_eq!(
        post(port, &first, &format!("{base}/configure"), settings).0,
        200
    );
    assert_eq!(
        post(port, &first, &format!("{base}/start"), json!({})).0,
        409,
        "changed rules require new consent"
    );
    assert_eq!(
        post(port, &second, &format!("{base}/ready"), json!({})).0,
        200
    );
    assert_eq!(
        post(port, &first, &format!("{base}/start"), json!({})).0,
        200
    );
    let preset = tokio::time::timeout(std::time::Duration::from_secs(10), presets.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(preset.house_rules.free_mulligan_count(), 2);
    assert_eq!(preset.seats[0].starting_life, Some(45));
    assert_eq!(preset.seats[1].starting_life, Some(30));
    assert_eq!(preset.seats[0].starting_battlefield.len(), 2);
    assert_eq!(
        preset.house_rules.starting_counters[0].counters[0].amount,
        3
    );
    assert_eq!(
        preset.prints[usize::from(preset.seats[0].starting_battlefield[0].print.get())].finish,
        baylee_core::preset::Finish::Foil
    );
    assert!(preset.seats.iter().all(|s| !s.capabilities.dev_commands));
    agent.abort();
}

/// A mixed table (owner, 10.10.2026): decks of different formats sit down
/// side by side. The room says `"format":"mixed"` in its setup, which the
/// listing carries to every client (so none warns about another format),
/// and the game is built as every room's is: each deck as itself, the
/// commander deck's leader in the command zone, the other deck without one,
/// and both at the room's own starting life.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_mixed_table_seats_a_commander_deck_beside_a_freeform_one() {
    let gateway = spawn_gateway("room-mixed");
    let port = gateway.port;
    let (agent, mut presets) = attach_agent_watching(&gateway).await;
    let host = login(port, "mixedhost", "MixedHost");
    let guest = login(port, "mixedguest", "MixedGuest");
    let pool = baylee_cards::pool::rows();
    let leader = pool
        .iter()
        .find(|c| c.commander && c.identity == "G")
        .expect("this pool has a green commander");
    let (status, saved) = post(
        port,
        &host,
        "/decks",
        json!({
            "name": "Leader",
            "cards": [format!("1 {}", leader.english_name), "59 Forest"],
            "commanders": [leader.english_name],
        }),
    );
    assert_eq!(status, 200, "{saved}");
    let leader_deck = json_field(&saved, "deck_id").to_string();
    let (status, saved) = post(
        port,
        &guest,
        "/decks",
        json!({"name":"Sixty", "cards":["40 Forest","20 Swamp"]}),
    );
    assert_eq!(status, 200, "{saved}");
    let sixty = json_field(&saved, "deck_id").to_string();

    let (status, body) = post(
        port,
        &host,
        "/lobby/games",
        json!({"mode":"open","seats":2,"deck_id":leader_deck}),
    );
    assert_eq!(status, 200, "{body}");
    let id = json_field(&body, "game_id").to_string();
    let base = format!("/lobby/games/{id}");
    let mixed = json!({"name":"Anything goes", "chairs":2, "setup":{
        "starting_life":20, "free_mulligans":1, "format":"mixed"
    }});
    assert_eq!(
        post(port, &host, &format!("{base}/configure"), mixed).0,
        200
    );
    assert_eq!(
        post(port, &guest, &format!("{base}/join"), json!({"seat":1})).0,
        200
    );
    assert_eq!(
        post(
            port,
            &guest,
            &format!("{base}/seats/1"),
            json!({"deck_id":sixty})
        )
        .0,
        200
    );
    let (_, listing) = http(
        port,
        "GET",
        &format!("/lobby/games?q={id}"),
        Some(&guest),
        "",
    );
    let listing: Value = serde_json::from_str(&listing).unwrap();
    let room = &listing["games"][0];
    assert_eq!(room["setup"]["format"], "mixed", "{room}");
    assert_eq!(room["seats"][0]["format"], "commander", "{room}");
    assert_eq!(room["seats"][1]["format"], "freeform", "{room}");
    for token in [&host, &guest] {
        assert_eq!(
            post(port, token, &format!("{base}/ready"), json!({})).0,
            200
        );
    }
    assert_eq!(
        post(port, &host, &format!("{base}/start"), json!({})).0,
        200
    );
    let preset = tokio::time::timeout(std::time::Duration::from_secs(10), presets.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(preset.seats[0].commanders.len(), 1, "the leader leads");
    assert!(preset.seats[1].commanders.is_empty());
    assert_eq!(preset.seats[0].deck.len(), 59, "and is not in the library");
    assert_eq!(preset.seats[1].deck.len(), 60);
    assert!(
        preset.seats.iter().all(|s| s.starting_life == Some(20)),
        "the room's life, for every deck"
    );
    agent.abort();
}

/// A room that never chose a format lists none, and a format word this
/// gateway does not know is read as the host's rather than refusing the
/// host's whole edit.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_room_without_a_format_lists_none_and_an_unknown_one_reads_as_the_hosts() {
    let gateway = spawn_gateway("room-format-word");
    let port = gateway.port;
    let host = login(port, "wordhost", "WordHost");
    let (status, body) = post(port, &host, "/lobby/games", json!({"mode":"open"}));
    assert_eq!(status, 200, "{body}");
    let id = json_field(&body, "game_id").to_string();
    let listed = || {
        let (_, listing) = http(
            port,
            "GET",
            &format!("/lobby/games?q={id}"),
            Some(&host),
            "",
        );
        let listing: Value = serde_json::from_str(&listing).unwrap();
        listing["games"][0]["setup"].clone()
    };
    assert!(listed().get("format").is_none(), "{}", listed());
    let newer = json!({"name":"x", "chairs":2, "setup":{"starting_life":20, "format":"pauper"}});
    assert_eq!(
        post(port, &host, &format!("/lobby/games/{id}/configure"), newer).0,
        200
    );
    assert!(listed().get("format").is_none(), "{}", listed());
    assert_eq!(listed()["starting_life"], 20);
}
