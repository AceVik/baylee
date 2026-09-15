use super::*;
use sea_orm::ActiveValue;

/// A store written before sideboards, confirmations, settings or languages
/// existed. Every one of those fields was added later, and a file from before
/// any of them is still a file somebody's account is in.
const ANCIENT: &str = r#"{
  "accounts": {
    "0192f0c0-0000-7000-8000-000000000001": {
      "id": "0192f0c0-0000-7000-8000-000000000001",
      "email": "Player@Example.COM",
      "display_name": "Player",
      "password_hash": "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA",
      "created_at": 1700000000
    }
  },
  "tokens": {},
  "decks": {
    "0192f0c0-0000-7000-8000-00000000000a": {
      "id": "0192f0c0-0000-7000-8000-00000000000a",
      "account_id": "0192f0c0-0000-7000-8000-000000000001",
      "name": "Mono Green",
      "cards": ["4 Llanowar Elves", "20 Forest"],
      "commander": null,
      "updated_at": 1700000100
    }
  }
}"#;

fn set<T: Into<sea_orm::Value>>(value: &ActiveValue<T>) -> &T {
    match value {
        ActiveValue::Set(v) => v,
        _ => panic!("the importer always sets every column"),
    }
}

/// The fields that did not exist when the file was written have to arrive as
/// their empty forms rather than stopping the read. A gateway that refused to
/// import a two-release-old store would be a gateway that lost the accounts
/// it had.
#[test]
fn a_file_from_before_half_the_fields_still_reads() {
    let legacy = read_legacy(ANCIENT).expect("an old store is still a store");
    let made = plan(&legacy, OffsetDateTime::UNIX_EPOCH);

    assert_eq!(made.tally().accounts, 1);
    assert_eq!(made.tally().decks, 1);
    assert_eq!(made.tally().orphans, 0);
    assert!(set(&made.accounts[0].confirmed_at).is_none());
    assert_eq!(set(&made.accounts[0].lang), "");
    assert!(set(&made.decks[0].sideboard).is_empty());
    assert!(set(&made.decks[0].sleeve).is_none());
}

/// The e-mail is written back exactly as its owner typed it. Uniqueness is an
/// index on `lower(email)`, so lowercasing the stored value would buy nothing
/// and would put an address in the confirmation mail that its owner does not
/// recognise.
#[test]
fn an_address_keeps_the_case_its_owner_typed() {
    let legacy = read_legacy(ANCIENT).unwrap();
    let made = plan(&legacy, OffsetDateTime::UNIX_EPOCH);
    assert_eq!(set(&made.accounts[0].email), "Player@Example.COM");
}

/// A deck whose owner is not in the file is dropped and counted. The old
/// format could hold one — nothing enforced the reference — and writing it
/// would fail the foreign key and take the whole import down with it.
#[test]
fn a_deck_with_no_owner_is_dropped_rather_than_failing_the_import() {
    let text = ANCIENT.replace(
        r#""account_id": "0192f0c0-0000-7000-8000-000000000001""#,
        r#""account_id": "0192f0c0-0000-7000-8000-0000000000ff""#,
    );
    let made = plan(&read_legacy(&text).unwrap(), OffsetDateTime::UNIX_EPOCH);

    assert_eq!(made.tally().accounts, 1, "the account still imports");
    assert_eq!(made.tally().decks, 0);
    assert_eq!(made.tally().orphans, 1);
}

/// A development store with a hand-typed id imports as an account with a
/// fresh one, and everything that pointed at the old spelling points at the
/// new id — not at a second, different one.
#[test]
fn a_hand_typed_id_is_minted_once_and_reused() {
    let text = ANCIENT.replace("0192f0c0-0000-7000-8000-000000000001", "dev");
    let made = plan(&read_legacy(&text).unwrap(), OffsetDateTime::UNIX_EPOCH);

    assert_eq!(made.tally().orphans, 0, "the deck still finds its owner");
    assert_eq!(set(&made.decks[0].account_id), set(&made.accounts[0].id));
    assert_eq!(
        set(&made.accounts[0].id).get_version_num(),
        7,
        "a minted id is a UUIDv7, like every id this gateway writes"
    );
}

/// A corrupted timestamp costs the row its date and nothing else. The
/// alternative — refusing the file — would lose an account over a field that
/// only says when a session lapses, and a session that far out of range has
/// lapsed either way.
#[test]
fn an_impossible_timestamp_does_not_cost_an_account() {
    let text = ANCIENT.replace("1700000000", &u64::MAX.to_string());
    let made = plan(&read_legacy(&text).unwrap(), OffsetDateTime::UNIX_EPOCH);

    assert_eq!(made.tally().accounts, 1);
    assert_eq!(
        set(&made.accounts[0].created_at),
        &OffsetDateTime::UNIX_EPOCH
    );
}

/// The file is kept, under a name that says what happened to it.
#[test]
fn the_file_is_moved_aside_and_never_deleted() {
    assert_eq!(
        imported_name(Path::new("gateway-store.json")),
        Path::new("gateway-store.json.imported")
    );
}

/// An empty store is a legitimate one — a gateway that has never been signed
/// in to. It must import as nothing rather than as an error.
#[test]
fn an_empty_store_imports_as_nothing() {
    let made = plan(&read_legacy("{}").unwrap(), OffsetDateTime::UNIX_EPOCH);
    assert!(made.tally().is_empty());
}

/// The old store kept a session's SHA-256 as hex and the column is `bytea`.
/// A hash that is not hex is a row nothing could ever match, so it is
/// dropped the way a token whose account is gone already was — never
/// imported as something else.
#[test]
fn a_hash_that_is_not_hex_is_dropped_rather_than_mangled() {
    assert_eq!(from_hex("00ff10"), Some(vec![0x00, 0xff, 0x10]));
    assert_eq!(from_hex(""), Some(Vec::new()));
    assert_eq!(from_hex("f"), None, "an odd length is not a byte string");
    assert_eq!(from_hex("zz"), None);
    assert_eq!(from_hex("00zz"), None);
    // The real shape: sixty-four characters for thirty-two bytes.
    assert_eq!(from_hex(&"a".repeat(64)).map(|b| b.len()), Some(32));
}
