use super::*;
use baylee_engine::choice::{PlayerAction, StandingAnswer};

fn ondu_cleric() -> u32 {
    baylee_cards::by_oracle_id("f4232466-dd6a-49bf-be6c-95905c3ded17")
        .expect("the card pool has Ondu Cleric")
        .index
        .get()
}

/// A stored answer has to arrive as the exact handle the engine keeps its
/// automation under. The two ends of that are now in different processes,
/// so the test drives both: what the gateway sends, read by the code the
/// engine reads it with.
#[test]
fn a_stored_answer_becomes_the_handle_the_engine_uses() {
    let stored = vec![
        store::StandingAnswer {
            card: ondu_cleric(),
            ability: 0,
            yes: true,
        },
        store::StandingAnswer {
            card: ondu_cleric(),
            ability: baylee_core::ids::AbilityRef::ENTERS,
            yes: false,
        },
    ];
    let actions = baylee_engine_server::standing_answers(&standing_payload(&stored));
    assert_eq!(
        actions,
        vec![
            PlayerAction::SetStandingAnswer {
                ability: baylee_core::ids::AbilityRef::new(
                    baylee_core::ids::CardIndex::new(ondu_cleric()),
                    0
                ),
                answer: Some(StandingAnswer::Yes),
            },
            PlayerAction::SetStandingAnswer {
                ability: baylee_core::ids::AbilityRef::new(
                    baylee_core::ids::CardIndex::new(ondu_cleric()),
                    baylee_core::ids::AbilityRef::ENTERS
                ),
                answer: Some(StandingAnswer::No),
            },
        ]
    );
}

/// The reserved indices address abilities that are not listed on the
/// card, so a round trip through the store and over the engine link must
/// not confuse them with ability 0.
#[test]
fn reserved_ability_handles_survive_the_store() {
    let stored = vec![store::StandingAnswer {
        card: ondu_cleric(),
        ability: baylee_core::ids::AbilityRef::MIRACLE,
        yes: true,
    }];
    let json = serde_json::to_string(&stored).expect("serializes");
    let back: Vec<store::StandingAnswer> = serde_json::from_str(&json).expect("round trips");
    assert_eq!(back, stored);
    let actions = baylee_engine_server::standing_answers(&standing_payload(&back));
    let PlayerAction::SetStandingAnswer { ability, .. } = &actions[0] else {
        panic!("expected a standing answer")
    };
    assert!(!ability.is_listed_ability());
}

/// A seat with no account — the house playing an empty chair — has no
/// remembered answers, and must not send the engine something it cannot
/// parse in place of that.
#[test]
fn no_answers_is_an_empty_list_the_engine_can_read() {
    assert!(baylee_engine_server::standing_answers(&standing_payload(&[])).is_empty());
    assert!(baylee_engine_server::standing_answers(b"[]").is_empty());
}

/// A store written before standing answers existed still loads.
///
/// The file is no longer where the gateway keeps anything — it is what a
/// first start against an empty database takes over — so the claim is the
/// importer's now, and `baylee_db::import` is where the rest of it is
/// tested. This stays because the property is the gateway's: a gateway
/// started against a two-release-old file must not lose the accounts in it.
#[test]
fn an_older_store_file_still_loads() {
    let old = r#"{"accounts":{},"tokens":{},"decks":{}}"#;
    let legacy = baylee_db::import::read_legacy(old).expect("older store loads");
    assert!(legacy.automation.is_empty());
}

fn deck_named(cards: &[&str], commander: Option<&str>) -> store::Deck {
    store::Deck {
        id: "d".into(),
        account_id: "a".into(),
        kind: baylee_db::entity::deck::KIND_ACCOUNT.into(),
        name: "a commander deck".into(),
        format: "commander".into(),
        description: None,
        origin: None,
        version: 1,
        cards: cards.iter().map(|s| (*s).to_string()).collect(),
        sideboard: Vec::new(),
        commanders: commander.map(ToString::to_string).into_iter().collect(),
        sleeve: None,
        playmat: None,
        updated_at: 0,
    }
}

/// A commander is one of the deck's rows on the way in — the builder
/// seats it among them on purpose — and exactly one card on the way out.
/// Copying it rather than moving it would shuffle a second Katara into
/// the library while the first one sat in the command zone.
#[test]
fn a_commander_is_moved_out_of_the_deck_list_and_not_copied() {
    let deck = deck_named(
        &["1 Katara, the Fearless", "3 Forest"],
        Some("Katara, the Fearless"),
    );
    let Ok(loaded) = loaded_deck(&deck) else {
        panic!("the rows and the name both resolve")
    };
    let katara = baylee_cards::decks::by_name("Katara, the Fearless").expect("in the pool");

    assert_eq!(loaded.commanders.len(), 1, "one commander");
    assert_eq!(loaded.commanders[0].index, katara);
    assert!(
        loaded.main.iter().all(|c| c.index != katara),
        "and no second copy left in the library"
    );
    assert_eq!(loaded.main.len(), 3, "the three Forests, and nothing else");
}

/// A deck posted straight to the API need not list its commander among
/// the rows, so a name that matches nothing there still seats one.
#[test]
fn a_commander_with_no_row_of_its_own_is_still_seated() {
    let deck = deck_named(&["3 Forest"], Some("Katara, the Fearless"));
    let Ok(loaded) = loaded_deck(&deck) else {
        panic!("the rows and the name both resolve")
    };
    assert_eq!(loaded.commanders.len(), 1, "one commander");
    assert_eq!(loaded.main.len(), 3, "and the deck list is untouched");
}

/// **Only a proxy this gateway was told about may speak for somebody else.**
///
/// `X-Forwarded-For` is a header anybody can set, so honouring it from any
/// peer lets a client rotate it per request and spend an unbounded number of
/// sign-in attempts: the limiter is keyed on whatever the header says.
#[test]
fn a_forwarded_address_is_read_only_from_a_proxy_on_the_list() {
    let proxy: IpAddr = "10.0.0.1".parse().expect("an address");
    let stranger: IpAddr = "203.0.113.9".parse().expect("an address");
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", "198.51.100.7".parse().expect("a value"));

    assert_eq!(rate_limit_ip(&[proxy], proxy, &headers), "198.51.100.7");
    assert_eq!(
        rate_limit_ip(&[proxy], stranger, &headers),
        "203.0.113.9",
        "a peer nobody vouched for is keyed on itself, whatever it claims"
    );
    assert_eq!(
        rate_limit_ip(&[], proxy, &headers),
        "10.0.0.1",
        "and an empty list vouches for nobody, which is the default"
    );
}

/// **The header is read from the right, because the left half is whatever
/// the client sent.**
///
/// A proxy either replaces `X-Forwarded-For` with the address it is talking
/// to, or appends that address to whatever arrived. Nothing in this
/// repository tells an operator which to configure, so the rule has to be
/// right for both — and on an appending proxy the client writes the left
/// half itself. Reading from the left hands the limiter a string the
/// attacker chose, which is the hole that trusting the header from anybody
/// had.
#[test]
fn the_client_is_the_rightmost_address_nobody_vouched_for() {
    let proxy: IpAddr = "10.0.0.1".parse().expect("an address");
    let inner: IpAddr = "10.0.0.9".parse().expect("an address");
    let key = |value: &str| {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", value.parse().expect("a value"));
        rate_limit_ip(&[proxy, inner], proxy, &headers)
    };

    assert_eq!(
        key("203.0.113.66, 198.51.100.7"),
        "198.51.100.7",
        "what the client wrote is on the left and what the proxy saw is on \
         the right"
    );
    assert_eq!(
        key("198.51.100.7"),
        "198.51.100.7",
        "a proxy that replaces the header writes one entry, and then the two \
         directions are the same answer"
    );
    assert_eq!(
        key(" 198.51.100.7 , 10.0.0.9 "),
        "198.51.100.7",
        "a hop that is itself on the list is stepped over, and the spacing a \
         proxy writes is not part of the address"
    );
    assert_eq!(
        key("10.0.0.9, 10.0.0.1"),
        "10.0.0.1",
        "a chain of nothing but vouched-for hops names no client, so the \
         peer is the key"
    );
}

/// What a proxy on the list did not send, or sent unreadably. Every one of
/// these ends at the peer, which is the strict direction: everybody behind
/// that proxy then shares one budget rather than none of them being counted.
#[test]
fn a_trusted_proxy_that_says_nothing_readable_is_keyed_on_itself() {
    let proxy: IpAddr = "10.0.0.1".parse().expect("an address");
    let key = |headers: &HeaderMap| rate_limit_ip(&[proxy], proxy, headers);

    assert_eq!(key(&HeaderMap::new()), "10.0.0.1", "no header at all");

    for value in ["", "   ", "not-an-address", "198.51.100.7, unknown"] {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", value.parse().expect("a value"));
        assert_eq!(
            key(&headers),
            "10.0.0.1",
            "{value:?} names no address this gateway can key on"
        );
    }
}

/// The list itself: an entry that is not an address is dropped, which makes
/// that proxy one nobody vouched for. Strict and silent, which is the right
/// way round for a list whose other failure switches the limiter off — and
/// it is why `10.0.0.0/8` does not work, since a range is not an address.
#[test]
fn a_proxy_entry_that_is_not_an_address_is_a_proxy_nobody_vouched_for() {
    assert_eq!(
        trusted_proxies("10.0.0.1, 2001:db8::1"),
        vec![
            "10.0.0.1".parse::<IpAddr>().expect("an address"),
            "2001:db8::1".parse::<IpAddr>().expect("an address"),
        ],
        "a comma-separated list, spacing and all"
    );
    assert!(trusted_proxies("").is_empty(), "unset trusts nobody");
    assert!(
        trusted_proxies("10.0.0.0/8, proxy.internal, 1.2.3.4:5678").is_empty(),
        "a range, a name and an address with a port are none of them an \
         address, and each of them is dropped rather than refused"
    );
}

/// **Three spellings shut the door, and every other value leaves it open.**
///
/// `registration_enabled` is what `/auth/config` tells a client and what
/// `POST /auth/register` checks, so this is the switch an operator reaches
/// for when they want a private gateway. `BAYLEE_REGISTRATION=no` leaves it
/// open. So does `OFF`, so does a variable exported empty, and none of them
/// says anything at the moment it is set — the operator finds out when
/// somebody registers.
///
/// Pinned as it is rather than as it should be, because widening the list
/// and refusing what is not on it are different decisions and neither is
/// this test's to make. The day one is taken, the second half of this test
/// is what changes: either the open list loses `no` and `OFF`, or an
/// unrecognised value stops being a value at all.
#[test]
fn an_unknown_value_leaves_registration_open_and_only_three_words_close_it() {
    for closed in ["off", "0", "false"] {
        assert!(
            !registration_enabled(Some(closed)),
            "{closed:?} is one of the three that shut it"
        );
    }
    for open in [
        None,
        Some(""),
        Some("no"),
        Some("OFF"),
        Some("Off"),
        Some("disabled"),
        Some("false "),
        Some("on"),
        Some("1"),
    ] {
        assert!(
            registration_enabled(open),
            "{open:?} leaves registration open, and an operator who wrote it \
             meant the opposite"
        );
    }
}

/// What a deck list of these lines parses to, or what the player is told.
///
/// Every refusal here is a 400 and that is folded in rather than asserted
/// per case: the deck is the body of the request, so there is no other
/// answer for a list this server cannot read.
fn parsed(lines: &[&str]) -> Result<usize, String> {
    let owned: Vec<String> = lines.iter().map(|line| (*line).to_string()).collect();
    parse_deck_lines(&owned)
        .map(|rows| rows.len())
        .map_err(|(status, body)| {
            assert_eq!(
                status,
                StatusCode::BAD_REQUEST,
                "a deck is the body of the request"
            );
            body.0.error.into_owned()
        })
}

/// **Four copies is a deck's limit, not a line's.**
///
/// That is the rule a per-line check gets wrong, and it gets it wrong in the
/// direction a player finds by accident rather than on purpose: three
/// Lightning Bolt and then two more is five Lightning Bolt, and every line
/// in that list is legal on its own. The count is carried per card across
/// the whole list, which is the only reason this function exists instead of
/// a loop at each of its two call sites.
#[test]
fn the_copy_limit_is_counted_over_the_deck_and_not_over_one_line() {
    const TOO_MANY: &str = "invalid card count (1-4, unlimited for basic lands)";

    assert_eq!(parsed(&["4 Lightning Bolt"]), Ok(1));
    assert_eq!(parsed(&["5 Lightning Bolt"]), Err(TOO_MANY.to_string()));
    assert_eq!(
        parsed(&["3 Lightning Bolt", "1 Lightning Bolt"]),
        Ok(2),
        "four in two rows is four"
    );
    assert_eq!(
        parsed(&["3 Lightning Bolt", "2 Lightning Bolt"]),
        Err(TOO_MANY.to_string()),
        "and five in two rows is five"
    );
    assert_eq!(
        parsed(&["1 Lightning Bolt"; 5]),
        Err(TOO_MANY.to_string()),
        "however many rows it is spread over"
    );
    assert_eq!(
        parsed(&["4 Lightning Bolt (LEA)", "4 Lightning Bolt (M10)"]),
        Err(TOO_MANY.to_string()),
        "two printings of one card are one card — the eight Bolts that went \
         through a per-row check once a printing became part of a row"
    );
    assert_eq!(
        parsed(&["4 Lightning Bolt", "4 Karakas"]),
        Ok(2),
        "and it is counted per card, so two cards are two counts"
    );
}

/// A basic land is the exemption, and what makes one is the **printing**:
/// the supertype and the type off the front face, never the name. A
/// Karakas is a legendary land and is four like everything else.
#[test]
fn a_basic_land_is_the_one_card_a_deck_may_hold_any_number_of() {
    assert_eq!(parsed(&["40 Forest"]), Ok(1));
    assert_eq!(
        parsed(&["20 Forest", "20 Island"]),
        Ok(2),
        "and each of them separately"
    );
    assert_eq!(
        parsed(&["5 Karakas"]),
        Err("invalid card count (1-4, unlimited for basic lands)".to_string()),
        "a land that is not basic is not exempt, whatever else it is"
    );
}

/// The list has a ceiling as well, and it is checked **as the lines are
/// read** rather than at the end, so a list that would take a gigabyte to
/// expand is refused at the row that passes 250 rather than after all of
/// them.
///
/// A count of nought never reaches here: `deckrow::parse` refuses it first
/// and the sentence a player gets is that parser's, which is why the check
/// standing behind it in this function cannot fire today. The assertion
/// below is on the message rather than on the refusal, so the day a nought
/// does get this far it says so here instead of silently changing which
/// half answered.
#[test]
fn the_ceiling_is_the_whole_list_and_is_read_row_by_row() {
    assert_eq!(parsed(&["250 Forest"]), Ok(1), "a deck of exactly the cap");
    assert_eq!(
        parsed(&["251 Forest"]),
        Err("deck too large".to_string()),
        "and one more is not a deck"
    );
    assert_eq!(
        parsed(&["200 Forest", "51 Island"]),
        Err("deck too large".to_string()),
        "the ceiling is the total and not the row"
    );
    assert_eq!(
        parsed(&["0 Forest"]),
        Err("malformed card count".to_string()),
        "nought is refused by the row parser, one layer before this one"
    );
}

/// **A card that exists and a card that does not are two different
/// refusals**, and the difference is the one a player can act on: a typo is
/// theirs to fix and a card this build compiles nothing for is not.
///
/// The name is looked for in the `CardIndex` ledger, which numbers every
/// card there is rather than this pool, so the unplayable card is found
/// here rather than written down — a name spelled into the test would
/// become a card one day and the test would go green having stopped asking
/// the question.
#[test]
fn a_card_this_build_cannot_play_is_refused_by_a_different_sentence() {
    let unplayable = baylee_cards_index::ROWS
        .iter()
        .find(|row| baylee_cards::decks::by_name(row.name).is_none())
        .expect("the ledger names more cards than this build compiles");

    assert_eq!(
        no_such_card(unplayable.name, "unknown card"),
        EXISTS_UNPLAYABLE
    );
    let row = format!("1 {}", unplayable.name);
    assert_eq!(
        parsed(&[row.as_str()]),
        Err(EXISTS_UNPLAYABLE.to_string()),
        "and that is the sentence the deck route gives it"
    );
    assert_eq!(
        no_such_card("Lihgtning Bolt", "unknown card"),
        "unknown card",
        "a name nothing has ever printed is the caller's own sentence"
    );

    assert_eq!(
        no_such_card_at(unplayable.index),
        EXISTS_UNPLAYABLE,
        "the same two facts reached by index"
    );
    assert_eq!(
        no_such_card_at(baylee_core::ids::CardIndex::new(
            u32::try_from(baylee_cards_index::ROWS.len()).expect("the ledger fits in a u32")
        )),
        "unknown card",
        "one past the last row is no card at all"
    );
}
