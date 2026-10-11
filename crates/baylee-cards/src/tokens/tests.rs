use super::*;
use crate::generated_tokens::GENERATED;

/// How many tokens this file declared when the ledger was seeded. The
/// list only grows, so a reader of it that finds fewer has stopped
/// reading rather than found a shorter file.
const HAND_WRITTEN_FLOOR: usize = 14;

/// Whether a token in [`ALL`] is one a reader wrote rather than one a
/// person did. Two tests need the distinction: a token a person wrote
/// chose a picture, and a generated one has none to choose from yet.
fn is_generated(token: &TokenDef) -> bool {
    GENERATED.iter().any(|g| std::ptr::eq(*g, token))
}

/// Every token this file declares has to be in the ledger, or it has no
/// id: [`token_id`] answers `u16::MAX` for it and the token reaches the
/// table nameless as far as art is concerned. The ledger is written by
/// `cargo xtask codegen`, which does not run in CI — so a token written
/// by hand and never followed by a codegen run is exactly the mistake
/// this catches, and nothing else would.
#[test]
fn every_token_written_here_has_an_id_in_the_ledger() {
    let declared: Vec<&str> = include_str!("../tokens.rs")
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("pub static ")?
                .strip_suffix(": TokenDef = TokenDef {")
        })
        .collect();
    assert!(
        declared.len() >= HAND_WRITTEN_FLOOR,
        "read {} tokens out of this file; the list only grows",
        declared.len()
    );
    // Against the ledger's own text, because what the ledger records is
    // a *constant* and a constant is a name: `ALL` holds the definitions
    // those names resolve to, and two tokens are allowed to be equal
    // there in a way two names never are.
    let ledger = include_str!("../generated_tokens.rs");
    let missing: Vec<&&str> = declared
        .iter()
        .filter(|name| !ledger.contains(&format!("&tokens::{name},")))
        .collect();
    assert!(
        missing.is_empty(),
        "these tokens have no id — run `cargo xtask codegen`: {missing:?}"
    );
    assert_eq!(
        declared.len() + GENERATED.len(),
        ALL.len(),
        "the ledger holds rows this file does not declare"
    );
}

/// No two rows of the ledger are the same token. Two equal definitions
/// are two ids for one permanent, and whichever of them a card happens
/// to name decides which picture it wears — a difference nothing in the
/// rules can see and nobody would think to look for.
#[test]
fn no_two_rows_of_the_ledger_are_the_same_token() {
    for (i, a) in ALL.iter().enumerate() {
        for b in &ALL[i + 1..] {
            assert!(a != b, "two ids for one token: {}", a.name);
        }
    }
}

/// The id is the art key, so it has to survive the round trip. A token
/// left out of [`ALL`] answers `u16::MAX` and reaches the client with no
/// identity at all — the failure this test exists to catch.
#[test]
fn every_token_answers_to_the_id_it_is_filed_under() {
    for (i, token) in ALL.iter().enumerate() {
        let id = token_id(token);
        assert_eq!(id as usize, i, "{} is misfiled", token.name);
        assert!(std::ptr::eq(by_token_id(id).expect("round trip"), *token));
    }
    assert!(by_token_id(u16::MAX).is_none());
}

/// Every card that creates a token must name one from this file. A
/// `TokenDef` literal in a card file compiles and works, but it has no
/// id, so the token loses its art the moment it reaches the table — the
/// bug Urza's Saga and Skyclave Apparition both carried.
///
/// Through [`crate::tests::every_card_file`], because this read `src/cards`
/// with a flat `read_dir` for as long as the taxonomy has existed — one
/// `mod.rs` and no cards, an empty worklist reported as a clean pool.
#[test]
fn no_card_file_defines_its_own_token() {
    let mut offenders = Vec::new();
    for (name, text) in crate::tests::every_card_file() {
        // The import line names the type without constructing one; a
        // literal is the `TokenDef {` that follows a `static` or `let`.
        if text.contains("TokenDef {") {
            offenders.push(name);
        }
    }
    offenders.sort();
    assert!(
        offenders.is_empty(),
        "these card files define tokens instead of using `crate::tokens`: {offenders:?}"
    );
}

/// A token with no picture is the flat coloured rectangle this file's
/// art keys exist to replace, and a *malformed* id is worse than none:
/// the client builds a URL out of it and fetches a guaranteed 404 every
/// time the token is drawn. Both are build failures, because neither is
/// visible from anywhere but the felt.
///
/// The shape checked here is the one `baylee_client_core::images` will
/// accept — a hyphenated 36-character UUID that is not the nil one.
///
/// Only the hand-written half. A generated token has no printing chosen
/// for it and carries an empty id deliberately, which `tokenart::of`
/// filters into "draw the face" — the one honest answer a reader that
/// never saw a picture can give. Choosing art for those is work this
/// test must not pretend has been done.
#[test]
fn every_hand_written_token_names_a_picture_the_client_can_fetch() {
    let mut checked = 0;
    for token in ALL.iter().filter(|t| !is_generated(t)) {
        checked += 1;
        let id = token.scryfall_id;
        // This exact independently described token has no matching printed art.
        if std::ptr::eq(*token, &raw const SNAKE_1_1_ARTIFACT_POISON) {
            assert!(id.is_empty(), "do not substitute art for a different Snake");
            continue;
        }
        assert!(!id.is_empty(), "{} has no art", token.name);
        assert_eq!(id.len(), 36, "{}: {id} is not a UUID", token.name);
        assert!(id.contains('-'), "{}: {id} is not a UUID", token.name);
        assert!(
            !id.chars().all(|c| c == '0' || c == '-'),
            "{}: the nil UUID is well-formed and always wrong",
            token.name
        );
        assert!(
            id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'),
            "{}: {id} is not hexadecimal",
            token.name
        );
    }
    assert!(
        checked >= HAND_WRITTEN_FLOOR,
        "checked {checked} hand-written tokens; the list only grows"
    );
}

/// Two tokens sharing a picture is not an error — a 1/1 and a 2/2
/// Shapeshifter could reasonably wear the same art — but it has never
/// been what was *meant* here, and a copied line is how it would happen.
#[test]
fn no_two_hand_written_tokens_were_given_the_same_picture_by_accident() {
    let mut ids: Vec<&str> = ALL
        .iter()
        .filter(|t| !is_generated(t))
        .map(|t| t.scryfall_id)
        .collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "two tokens share an art id");
}

/// A token whose name promises a sacrifice outlet and delivers nothing is
/// worse than no token: the player sees a Treasure and cannot spend it.
#[test]
fn the_artifact_tokens_all_carry_their_printed_ability() {
    for token in [&TREASURE, &CLUE, &FOOD, &BLOOD] {
        assert!(
            !token.abilities.is_empty(),
            "{} has no ability to activate",
            token.name
        );
    }
}
