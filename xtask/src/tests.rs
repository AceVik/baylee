/// Every deck a dev table names is found in one of the two files; the
/// owner's play decks sit down with what the pool has, their printings
/// stripped in the file a bridge reads, and the acceptance decks lose
/// nothing.
#[test]
fn a_dev_table_finds_decks_in_both_files() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in <workspace>/xtask");
    let names = super::table_deck_names(root);
    for name in [
        "Allytifact",
        "Victory",
        "Euro-Highlander",
        "Schwarzrand",
        "Weltenbaum",
    ] {
        assert!(names.iter().any(|n| n == name), "{name} in {names:?}");
        let deck = super::table_deck(root, name).unwrap();
        let acceptance = name == "Allytifact" || name == "Victory";
        assert_eq!(deck.acceptance, acceptance, "{name}");
        if acceptance {
            assert!(deck.missing.is_empty(), "{name}: {:?}", deck.missing);
        }
        let text = deck.text();
        assert!(
            !text.contains(" [de]") && !text.contains(" (IKO) "),
            "{name}"
        );
        // The file a bridge gets is read whole, as `baylee-seat --deck`
        // reads it.
        let Ok(baylee_deckio::Import::Read { read, .. }) = baylee_deckio::import(&text) else {
            panic!("{name}: the bridge's file does not read as a deck");
        };
        assert!(read.skipped.is_empty(), "{name}: {:?}", read.skipped);
        let stored = read.document.stored();
        assert_eq!(stored.name.as_deref(), Some(name));
        assert_eq!(
            !stored.commanders.is_empty(),
            deck.body["commander"].is_string()
        );
    }
    assert!(super::table_deck(root, "Nope").is_err());
}

#[test]
fn a_temporary_special_action_is_an_optional_offer() {
    let payload = serde_json::json!({
        "oracle_text": "Until end of turn, you may pay 1 life any time you could activate a mana ability."
    });
    let mut tally = super::PrintingTally::default();
    let mut problems = 0;
    for (name, expected) in [("Channel", 0), ("Guardian Angel", 0), ("Fog", 1)] {
        let def = baylee_cards::decks::by_name(name)
            .and_then(baylee_cards::by_index)
            .expect("fixture is registered");
        super::check_optional_clauses_are_offered(
            "fixture",
            def,
            &payload,
            &mut tally,
            &mut problems,
        );
        assert_eq!(problems, expected, "{name}");
    }
    assert_eq!(tally.optional_clauses, 3);
}

#[test]
fn a_mana_spending_permission_needs_no_separate_may_question() {
    let payload = serde_json::json!({
        "oracle_text": "You may spend white mana as though it were red mana."
    });
    let mut tally = super::PrintingTally::default();
    let mut problems = 0;
    for (name, expected) in [("Sunglasses of Urza", 0), ("Fog", 1)] {
        let def = baylee_cards::decks::by_name(name)
            .and_then(baylee_cards::by_index)
            .expect("fixture is registered");
        super::check_optional_clauses_are_offered(
            "fixture",
            def,
            &payload,
            &mut tally,
            &mut problems,
        );
        assert_eq!(problems, expected, "{name}");
    }
    assert_eq!(tally.optional_clauses, 2);
}

/// "You control enchanted creature" is the change of control, not a
/// filter the card forgot: Control Magic passes the scope check with no
/// exception written for it, and a card that says "you control" with
/// nothing behind it still does not.
#[test]
fn a_change_of_control_is_the_you_control_the_text_says() {
    let payload = serde_json::json!({
        "oracle_text": "Enchant creature\nYou control enchanted creature."
    });
    let def = baylee_cards::decks::by_name("Control Magic")
        .and_then(baylee_cards::by_index)
        .expect("Control Magic is in the pool");
    let mut problems = 0;
    super::check_scope_matches_the_text(
        "control_magic",
        "Not An Exception",
        def,
        &payload,
        &mut problems,
    );
    assert_eq!(problems, 0);

    let fog = baylee_cards::decks::by_name("Fog")
        .and_then(baylee_cards::by_index)
        .expect("Fog is in the pool");
    super::check_scope_matches_the_text("fog", "Not An Exception", fog, &payload, &mut problems);
    assert_eq!(problems, 1, "no filter and no change of control");
}

/// A batch is allowed to write the cards it asked for and nothing else.
///
/// Codegen rewrites every machine-owned card on every run, so a rewrite is
/// not news: an identical one leaves no diff. A card that comes out
/// *modified* and is none of the ones being added is a printing or a
/// reader that moved underneath the batch — almost always a Scryfall
/// header, which is `xtask refresh-oracle`'s job — and it has no business
/// riding into a commit about new cards.
#[test]
fn a_batch_reports_only_the_cards_it_did_not_ask_for() {
    let asked: std::collections::BTreeSet<String> = ["Yavimaya Coast", "Mox Opal"]
        .iter()
        .map(|n| super::stubgen::slug(n))
        .collect();
    let porcelain = concat!(
        "?? crates/baylee-cards/src/cards/lands/pain/yavimaya_coast.rs\n",
        " M crates/baylee-cards/src/cards/artifacts/mv_0/mox_opal.rs\n",
        " M crates/baylee-cards/src/cards/instants/mv_1/swords_to_plowshares.rs\n",
        "R  crates/baylee-cards/src/cards/a.rs -> crates/baylee-cards/src/cards/lands/utility/karakas.rs\n",
    );
    let drift = super::drift_from_status(porcelain, &asked);
    assert_eq!(
        drift,
        vec![
            "crates/baylee-cards/src/cards/instants/mv_1/swords_to_plowshares.rs".to_string(),
            "crates/baylee-cards/src/cards/lands/utility/karakas.rs".to_string(),
        ],
        "the new card is this batch's own output and the asked-for rewrite is \
         expected; the other two are somebody else's change arriving inside it"
    );
}

use super::{
    PrintedBound, PrintedMana, header_scryfall_id, knob, one_row_per_token,
    printed_enters_tapped_bound, printed_mana_offered, printed_subtypes, quoted_value,
    refuse_twin_names, script_for,
};
use baylee_cards_codegen::{tokengen, tokenledger};
use baylee_core::generated::subtypes;
use std::collections::BTreeMap;

/// A batch appends a name once, whatever case the pool spells it in, and
/// never twice from one proposal.
///
/// This is the guard in front of #47: two pool lines naming one card emit
/// the module twice, and the tree that results cannot be repaired by
/// `xtask` — it cannot run. The pool is hand-edited, so a spelling that
/// differs only in case is a real shape and not a hypothetical.
#[test]
fn a_name_the_pool_already_holds_is_not_appended_again() {
    let pool = "# a comment\nLightning Bolt\n  brainstorm  \n\n";
    let taken = super::pool_additions(
        pool,
        &[
            "Lightning Bolt",
            "Brainstorm",
            "Counterspell",
            "Counterspell",
            "# a comment",
        ],
    );
    assert_eq!(taken, vec!["Counterspell".to_string()]);
    // And an empty pool takes everything, in the order it was proposed.
    assert_eq!(
        super::pool_additions("", &["Swords to Plowshares", "Ancestral Recall"]),
        vec![
            "Swords to Plowshares".to_string(),
            "Ancestral Recall".to_string()
        ]
    );
}

/// The committed subtype table is what `codegen` writes for the catalogs
/// it stands for — byte for byte, *after* the rustfmt every generated
/// `.rs` goes through on its way to disk.
///
/// Two claims in one assertion. The file really is generated output, so
/// nobody has hand-edited it; and a run that changes nothing renumbers
/// nothing, which is the whole of #43. It lives here rather than beside
/// the emitter because `write_or_check` is what formats, and an emitter
/// output compared *unformatted* passes while `cargo fmt --all` and
/// `codegen --check` undo each other forever — which is exactly what
/// happened the first time: rustfmt folded the one-entry battle mask onto
/// one line and the two tools disagreed.
///
/// The catalogs are read back out of the compiled table, so this needs no
/// network and keeps holding after Scryfall prints a new subtype and
/// `codegen` appends it.
#[test]
fn the_committed_subtype_table_is_what_codegen_writes_for_it() -> anyhow::Result<()> {
    use baylee_cards_codegen::catalog::{PriorSubtypes, SubtypeCatalogs, render_subtypes_rs};

    let mut cats = SubtypeCatalogs::default();
    for raw in 0..subtypes::COUNT {
        let id = baylee_core::ids::SubtypeId::new(raw);
        let name = subtypes::name(id).expect("every id is named").to_string();
        let kind = subtypes::kind(id).expect("every id has a kind");
        for (k, list) in cats.ordered_mut() {
            if k == kind {
                list.push(name);
                break;
            }
        }
    }
    cats.normalize();

    let rendered = super::format_rust(&render_subtypes_rs(
        &cats,
        &PriorSubtypes::from_compiled_table(),
    ))?;
    let committed = include_str!("../../crates/baylee-core/src/generated/subtypes.rs").to_string();
    assert_eq!(
        rendered, committed,
        "the generated subtype table is not what codegen would write"
    );
    Ok(())
}

/// The mana check holds the code against the symbols a printing spells,
/// and a printing that names its mana in words spells none — so the
/// question "which colours does this offer" has to be *declined* rather
/// than answered with an empty list. Black Lotus is the card that found
/// the hole: "any one color" is not "any color", it reached the symbol
/// scan, and five colours were reported as unprinted.
#[test]
fn mana_promised_in_words_is_declined_and_not_read_as_no_mana() {
    for printed in [
        "{t}, sacrifice this artifact: add three mana of any one color.",
        "{t}: add one mana of any color.",
        "{t}: add one mana of any type that a land you control could produce.",
        "{t}: add two mana in any combination of colors.",
        "{t}: add one mana of any of the exiled card's colors.",
    ] {
        assert!(
            matches!(printed_mana_offered(printed), PrintedMana::InWords),
            "{printed:?} names its mana in words"
        );
    }
    // The one printed form that says "any" *and* spells symbols is read,
    // because those symbols are exactly what the check compares against.
    assert!(matches!(
        printed_mana_offered("{t}: add x mana in any combination of {r} and/or {w}."),
        PrintedMana::Symbols(ref s) if s == &['r', 'w']
    ));
    // An ordinary land, and a card with no "add" at all.
    assert!(matches!(
        printed_mana_offered("{t}: add {u} or {r}."),
        PrintedMana::Symbols(ref s) if s == &['u', 'r']
    ));
    assert!(matches!(
        printed_mana_offered("{t}: draw a card."),
        PrintedMana::Nothing
    ));
}

/// A card is allowed a quotation mark in its own name, and the reader
/// that stops at the first one reports the header it agrees with as a
/// mismatch.
///
/// Both directions, because the repair is only a repair if the ordinary
/// name still comes back the same: seven ledger names carry an escape
/// and 33 687 do not.
#[test]
fn a_name_that_prints_a_quotation_mark_is_read_whole() {
    let line = r#"        name = "Kongming, \"Sleeping Dragon\"","#;
    assert_eq!(
        knob(line, "name").and_then(|v| quoted_value(v, "\"")),
        Some("Kongming, \"Sleeping Dragon\"".to_string())
    );
    let line = r#"        name = "Lightning Bolt","#;
    assert_eq!(
        knob(line, "name").and_then(|v| quoted_value(v, "\"")),
        Some("Lightning Bolt".to_string())
    );
    // A literal that never closes is not a name; the old reader would
    // have answered with whatever came after it.
    assert_eq!(quoted_value("name = \"unterminated", "\""), None);
}

/// `reach-list` looks a script up under two spellings of one card, and a
/// tier that matched nothing would leave every two-faced card out of the
/// worklist without saying so.
#[test]
fn a_script_is_found_by_the_whole_name_and_by_the_front_face() {
    let index: BTreeMap<String, String> = [
        ("Fire // Ice".to_string(), "f/fire_ice.txt".to_string()),
        ("Delver of Secrets".to_string(), "d/delver.txt".to_string()),
    ]
    .into_iter()
    .collect();
    // The whole name wins wherever the corpus spells it that way.
    assert_eq!(
        script_for(&index, "Fire // Ice").map(String::as_str),
        Some("f/fire_ice.txt")
    );
    // And the front face answers where it does not: the ledger follows
    // Scryfall and writes both halves, the corpus one of them.
    assert_eq!(
        script_for(&index, "Delver of Secrets // Insectile Aberration").map(String::as_str),
        Some("d/delver.txt")
    );
    // A card nobody scripted is a miss and never the nearest thing.
    assert_eq!(script_for(&index, "Black Lotus"), None);
}

/// The id in a `//! Set:` line is what decides which printing a card is,
/// so reading it is worth a test rather than a spelling: the line carries
/// two ids separated by `|`, and a reader that stopped at the space would
/// take the `|` with it and match no cached file.
#[test]
fn the_set_header_yields_the_printing_id_and_not_the_oracle_one() {
    let line = baylee_cards_codegen::stubgen::set_line(
        "msc",
        "211",
        "Marvel Super Heroes Commander",
        "91fdb56b-54d5-4272-8319-505ff987fe9b",
        "6ad8011d-3471-4369-9d68-b264cc027487",
    );
    assert_eq!(
        header_scryfall_id(&line),
        Some("91fdb56b-54d5-4272-8319-505ff987fe9b")
    );
    assert_eq!(header_scryfall_id("//! Set: MSC #211\n"), None);
    assert_eq!(
        header_scryfall_id("pub static CARD: CardDef = card!(\n"),
        None
    );
}

/// The pool names a card by its front face or the way the ledger names
/// it, and both slug to one file. Without this the run says `codegen
/// complete` and leaves `cards/mod.rs` declaring the module twice — which
/// `codegen` then cannot repair, because `xtask` links `baylee-cards`.
#[test]
fn one_card_written_two_ways_is_refused_before_anything_is_written() {
    let ok = ["Hengegate Pathway".to_string(), "Island".to_string()];
    refuse_twin_names(&ok).expect("two different cards are two cards");

    let twins = [
        "Hengegate Pathway".to_string(),
        "Hengegate Pathway // Mistgate Pathway".to_string(),
        "Island".to_string(),
    ];
    let err = refuse_twin_names(&twins).expect_err("one card named twice");
    let message = err.to_string();
    assert!(message.contains("hengegate_pathway"), "{message}");
    assert!(message.contains("Mistgate"), "names both lines: {message}");
    assert!(!message.contains("Island"), "and nothing else: {message}");
}

/// A lobby that answers `expected` requests with `200 {}` and hands back
/// what it was asked: the request line and the JSON body of each.
fn fake_lobby(
    expected: usize,
) -> (
    std::net::SocketAddr,
    std::thread::JoinHandle<Vec<(String, serde_json::Value)>>,
) {
    use std::io::{BufRead, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        // Sixty seconds, and it is a hang-breaker rather than a timing
        // assumption. It was three, which is the length of time this
        // thread needs to be *scheduled* — not the length of the
        // conversation, which is three local requests and takes
        // milliseconds. Under `cargo test` xtask's binary has the machine
        // largely to itself and three was invisible; under `cargo nextest
        // run`, where every test is its own process and ten run at once,
        // the loop reached its deadline before the first request was
        // served, dropped the listener, and the client reported the only
        // thing it could see: `Peer disconnected`. A deadline that a
        // loaded machine can miss is a test that fails for a reason that
        // is not about the code.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while requests.len() < expected && std::time::Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            };
            // macOS hands the accepted socket its listener's `O_NONBLOCK`;
            // Linux does not. Measured here rather than recalled: a probe
            // on this machine reported the accepted socket non-blocking
            // after the listener was set so. `set_read_timeout` writes
            // `SO_RCVTIMEO` and clears no flag, so on a socket that
            // inherited the flag every read below answers `WouldBlock` the
            // instant no byte has arrived yet — which is what failed, once
            // in twenty-five runs, twenty-one milliseconds into a read
            // whose timeout says ten seconds. The earlier reading of that
            // failure as scheduling latency was wrong: a ten-second
            // timeout that expires in twenty-one milliseconds is not a
            // deadline being missed, it is a deadline that was never
            // armed.
            stream.set_nonblocking(false).unwrap();
            // Ten rather than two: this bounds how long one request's
            // bytes may take to arrive over loopback, and under load that
            // is scheduling latency, not I/O.
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            requests.push((
                request,
                serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            ));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .unwrap();
        }
        requests
    });
    (address, server)
}

#[test]
fn a_two_seat_dev_table_configures_the_requested_ai_before_starting() {
    let (address, server) = fake_lobby(3);
    super::arrange_room(
        &ureq::Agent::new_with_defaults(),
        &format!("http://{address}"),
        "test",
        "game",
        2,
        "expert",
        &[],
        &[],
        || Ok(()),
    )
    .unwrap();
    let requests = server.join().unwrap();
    assert_eq!(
        requests.len(),
        3,
        "duels must arrange a chair, say ready, then start"
    );
    assert!(requests[0].0.starts_with("POST /lobby/games/game/seats/1 "));
    assert_eq!(requests[0].1["ai"], "expert");
    assert!(requests[1].0.starts_with("POST /lobby/games/game/ready "));
    assert!(requests[2].0.starts_with("POST /lobby/games/game/start "));
}

/// `--bridge profile:<name>` plays a profile of the bridge's settings
/// file; anything else names its mind.
#[test]
fn a_bridge_plays_a_profile_or_a_mind() {
    assert_eq!(super::bridge_mind("profile:opus"), ["--profile", "opus"]);
    assert_eq!(super::bridge_mind("anthropic"), ["--mind", "anthropic"]);
    let model = "openai:deepseek-chat";
    assert_eq!(super::bridge_mind(model), ["--mind", model]);
}

/// A bridge's chair is left to the bridge, and the table is not said
/// ready before the bridge has sat down.
#[test]
fn a_dev_table_with_a_bridge_leaves_its_chair_open_and_starts_after_it() {
    let (address, server) = fake_lobby(3);
    let requests_before_start = std::cell::Cell::new(None);
    super::arrange_room(
        &ureq::Agent::new_with_defaults(),
        &format!("http://{address}"),
        "test",
        "game",
        3,
        "expert",
        &[],
        &[1],
        || {
            requests_before_start.set(Some(()));
            Ok(())
        },
    )
    .unwrap();
    assert!(
        requests_before_start.get().is_some(),
        "the bridge was waited for"
    );
    let requests = server.join().unwrap();
    let lines: Vec<&str> = requests.iter().map(|(line, _)| line.as_str()).collect();
    assert_eq!(requests.len(), 3, "{lines:?}");
    assert!(
        lines[0].starts_with("POST /lobby/games/game/seats/2 "),
        "{lines:?}"
    );
    assert!(
        lines[1].starts_with("POST /lobby/games/game/ready "),
        "{lines:?}"
    );
    assert!(
        lines[2].starts_with("POST /lobby/games/game/start "),
        "{lines:?}"
    );
}

/// A bridge that never sits down leaves the room arranged and unstarted:
/// nothing is said ready before it has.
#[test]
fn a_dev_table_whose_bridge_never_sits_down_is_not_started() {
    let (address, server) = fake_lobby(1);
    let error = super::arrange_room(
        &ureq::Agent::new_with_defaults(),
        &format!("http://{address}"),
        "test",
        "game",
        3,
        "expert",
        &[],
        &[1],
        || Err(anyhow::anyhow!("the bridge did not sit down")),
    )
    .expect_err("no start without the bridge");
    assert!(error.to_string().contains("did not sit down"), "{error}");
    let requests = server.join().unwrap();
    let lines: Vec<&str> = requests.iter().map(|(line, _)| line.as_str()).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("POST /lobby/games/game/seats/2 "),
        "{lines:?}"
    );
}

/// The branch the pool does not reach, and the reason it is written.
///
/// [`printed_subtypes`] is the honesty gate on
/// [`check_type_line_matches_the_printing`]: a word the catalog does not
/// know skips the face, so what counts as "known" decides which faces are
/// compared at all. "Time Lord" is the one subtype in the whole catalog
/// spelled as two words, no card in this pool prints it, and deleting the
/// longest-match pass therefore changes nothing `validate` says today —
/// which is exactly why the pass needs a test rather than a mutant. The
/// first Doctor Who card in the pool would otherwise read as two words
/// that name no subtype, and the whole face would go unchecked.
/// Two scripts that read as the same token are one row; two that read as
/// *different* tokens under one name are two, so the ledger can refuse
/// them.
///
/// The second half is the one that has to be a test. A set keyed on the
/// constant passes the first half perfectly and silently discards the
/// second body, which leaves `tokenledger::assign`'s collision refusal
/// unreachable from a `codegen` run — a guard that cannot fire, with
/// three files saying it does. `b_1_1_skeleton` beside
/// `b_1_1_skeleton_regenerate` is the reference's own shape.
#[test]
fn two_stems_are_one_row_only_when_they_read_as_one_token() {
    let body = |constant: &str, literal: &str| tokengen::TokenBody {
        constant: constant.to_string(),
        doc: "1/1 black Skeleton.".to_string(),
        literal: literal.to_string(),
        modules: vec!["creature".to_string()],
    };
    let plain = body("SKELETON_1_1_BLACK", "TokenDef { a }");
    let same = body("SKELETON_1_1_BLACK", "TokenDef { a }");
    let regenerating = body("SKELETON_1_1_BLACK", "TokenDef { a, abilities: b }");

    let one = one_row_per_token(vec![plain.clone(), same].into_iter());
    assert_eq!(one.len(), 1, "one token read twice is one row");

    let two = one_row_per_token(vec![plain, regenerating].into_iter());
    assert_eq!(
        two.len(),
        2,
        "two definitions at one name both reach the ledger"
    );
    assert_eq!(
        tokenledger::assign(Vec::new(), &[], &two),
        Err(tokenledger::LedgerError::Collision(
            "SKELETON_1_1_BLACK".to_string()
        )),
        "and the ledger is what refuses them"
    );
}

#[test]
fn a_two_word_subtype_is_one_subtype() {
    assert_eq!(
        printed_subtypes("Legendary Creature \u{2014} Time Lord"),
        Some(vec![subtypes::creature::TIME_LORD]),
    );
}

/// A type line with no dash prints no subtypes, and that is an answer.
///
/// Not a refusal: "this card has none" is a line worth comparing, and it
/// is the half Raffine's Tower needed — a `TypeSet::LAND` with an empty
/// subtype list against a printing that names three.
#[test]
fn a_type_line_with_no_dash_names_no_subtypes() {
    assert_eq!(printed_subtypes("Artifact"), Some(Vec::new()));
}

/// A word the catalog does not know refuses the whole line.
///
/// The catalogs are what `codegen` builds the constants from, so an
/// unknown word is this command's own gap and never a fact about the
/// card. `PrintingTally::type_lines` is what keeps a refusal from being
/// silent.
#[test]
fn an_unknown_word_refuses_the_line_rather_than_guessing() {
    assert_eq!(printed_subtypes("Creature \u{2014} Wizard Nonesuch"), None);
}

/// The two sentences the pool prints, read straight.
#[test]
fn an_unless_you_control_sentence_states_its_bound_as_printed() {
    assert_eq!(
        printed_enters_tapped_bound(
            "This land enters tapped unless you control two or more other lands."
        ),
        Some(PrintedBound::AtLeast(2))
    );
    assert_eq!(
        printed_enters_tapped_bound(
            "This land enters tapped unless you control two or fewer other lands."
        ),
        Some(PrintedBound::AtMost(2))
    );
    assert_eq!(
        printed_enters_tapped_bound(
            "This land enters tapped unless you control three or more other Islands."
        ),
        Some(PrintedBound::AtLeast(3))
    );
}

/// And the third, which is the second turned round.
///
/// "Tapped from two upwards" is "untapped at one and below", so the
/// number the card prints is one higher than the number the code holds.
/// Reading it straight is the defect this whole check was written for:
/// Lair of the Hydra wrote `at_most: 2` where its four cyclemates wrote
/// `at_most: 1`, and came down untapped off exactly two other lands.
#[test]
fn an_if_you_control_sentence_is_the_same_bound_one_lower() {
    assert_eq!(
        printed_enters_tapped_bound(
            "If you control two or more other lands, this land enters tapped."
        ),
        Some(PrintedBound::AtMost(1))
    );
}

/// "Unless you control" is an ordinary clause, and a card is allowed to
/// print one that has nothing to do with how it arrives.
///
/// The scan is per sentence for exactly this: over the whole text these
/// two lines would read as one, and the Bog would be reported as a land
/// whose coded bound is missing.
#[test]
fn an_unrelated_unless_clause_states_no_arrival_bound() {
    assert_eq!(
        printed_enters_tapped_bound(
            "This land enters tapped. When it enters, sacrifice it unless you \
             control two or more Swamps."
        ),
        None
    );
    assert_eq!(printed_enters_tapped_bound("{T}: Add {G}."), None);
}
