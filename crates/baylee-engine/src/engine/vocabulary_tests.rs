//! No card may name a DSL variant the engine parses and never acts on.
//!
//! The companion to `keyword_tests`' [`ENFORCED`] table, one level down: a
//! keyword is a bit and a card can claim one no rule reads, and the same is
//! true of an `Effect`, a `Trigger` or a `Modifier`. Such a card compiles,
//! passes `xtask validate`, is offered to the deckbuilder as `Implemented`,
//! and does nothing when it resolves.
//!
//! This module is short on purpose, because the compiler holds almost all of
//! it. `resolve::exec_immediate`'s match over `Effect` is **exhaustive** — no
//! `_` arm — so a variant with no rule at all cannot be added without a build
//! failure, and the same is now true of `layers::could_change_match` over
//! `Filter` and of the damage dispatch over `TargetSpec`. What the compiler
//! cannot see is an arm that exists and does nothing, which is the whole of
//! what [`SILENT`] lists.
//!
//! [`ENFORCED`]: super::keyword_tests

/// DSL vocabulary the engine parses and does not act on, with the site that
/// would have to read it.
///
/// This list is meant to shrink. An entry is a promise that no card in the
/// pool reaches it, so implementing the variant means taking its row out
/// rather than leaving it here as documentation.
const SILENT: &[(&str, &str)] = &[(
    "GrantSubtype",
    "resolve/mod.rs, `Effect::GrantSubtype { .. } => None` — the arm exists \
     and returns without touching the state, waiting on the continuous-effect \
     work M2 left behind",
)];

/// Ties every [`SILENT`] row to the variant it names, so renaming or removing
/// one is a compile error here.
///
/// A row whose variant no longer exists is worse than no row at all: the gate
/// stays green while matching nothing, and reads as a checked claim.
#[allow(dead_code)]
fn silent_variants_still_exist(effect: baylee_cards_dsl::Effect) -> bool {
    matches!(effect, baylee_cards_dsl::Effect::GrantSubtype { .. })
}

/// Whether `dump` names `variant` as a whole word, so `Scry` does not answer
/// for `ScryFor`.
fn names(dump: &str, variant: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    dump.match_indices(variant).any(|(at, _)| {
        dump[..at].chars().next_back().is_none_or(|c| !word(c))
            && dump[at + variant.len()..]
                .chars()
                .next()
                .is_none_or(|c| !word(c))
    })
}

/// The pool, read through `Debug`.
///
/// Every DSL enum derives it, and it recurses — so one `format!` per list
/// reaches a nested `then` branch, the abilities printed on a `TokenDef`
/// inside a `CreateToken`, and an emblem's, none of which a hand-written
/// walker would visit unless it had been told about each of them. Proven
/// rather than assumed: with `ManaSource::Choice` in [`SILENT`] the walk
/// names Smothering Tithe, whose own file writes no such thing — it makes a
/// Treasure, and the Treasure taps for a chosen color.
///
/// The card-level list and every face's own list are read separately, the
/// way `baylee_cards::lints` reads them, rather than through
/// `CardDef::abilities_for_face`: face 0 *shadows* the card-level list when
/// it states abilities of its own, so a card that filled both would have
/// half of itself unread here.
#[test]
fn no_card_names_a_dsl_variant_the_engine_does_not_read() {
    for (oracle_id, def) in baylee_cards::generated::ALL {
        let faces = def.faces.iter().map(|f| f.abilities);
        for list in core::iter::once(def.abilities).chain(faces) {
            let dump = format!("{list:?}");
            for (variant, site) in SILENT {
                assert!(
                    !names(&dump, variant),
                    "{} ({oracle_id}) names `{variant}`, which the engine \
                     parses and never acts on ({site}); implement it and take \
                     the row out of SILENT, or take the effect off the card",
                    def.name(),
                );
            }
        }
    }
}

/// The tokens the recursion above cannot reach.
///
/// The `Debug` dump follows a `CreateToken` into the `TokenDef` it carries,
/// which covers every token some card makes — and `tokens::ALL` holds three
/// that no card makes at all. Blood, Clue and Food are defined, given stable
/// ids and left waiting for the card that will create one, so their
/// abilities are read by nothing here: the walk starts at the card registry,
/// and `tokens.rs` sits beside `cards/`.
///
/// That door is worth a test of its own rather than a note, because it has
/// already swallowed one defect —
/// `offer_tests::no_token_carries_an_ability_the_engine_will_never_offer`
/// exists because a pool-wide grep scoped to `cards/` reported Recurring
/// Nightmare as the only card of its class while the Blood token sat one
/// directory up carrying the same cost.
#[test]
fn no_token_names_a_dsl_variant_the_engine_does_not_read() {
    for token in baylee_cards::tokens::ALL {
        let dump = format!("{:?}", token.abilities);
        for (variant, site) in SILENT {
            assert!(
                !names(&dump, variant),
                "the {} token names `{variant}`, which the engine parses and \
                 never acts on ({site}); implement it and take the row out of \
                 SILENT, or take the effect off the token",
                token.name,
            );
        }
    }
}

/// The word test, both ways round — the assertion above is worthless if this
/// is wrong in the permissive direction, and noisy if it is wrong in the
/// other.
#[test]
fn a_variant_name_is_matched_as_a_whole_word() {
    assert!(names("[GrantSubtype { subtype: 3 }]", "GrantSubtype"));
    assert!(names("Some(GrantSubtype)", "GrantSubtype"));
    assert!(names("GrantSubtype", "GrantSubtype"));
    assert!(!names("[GrantSubtypeAll { .. }]", "GrantSubtype"));
    assert!(!names("[MyGrantSubtype]", "GrantSubtype"));
    assert!(!names("[Scry { n: 1 }]", "GrantSubtype"));
}
