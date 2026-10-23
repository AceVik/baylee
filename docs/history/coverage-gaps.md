# Coverage gaps outside the engine's card tests

Measured on 2026-10-01 **without a coverage tool**: neither `cargo llvm-cov` nor
`cargo tarpaulin` is installed, so every state below is an estimate from reading
the public functions and branches against the tests that name them. Install
`cargo-llvm-cov` (owner's call) and replace the "state" column with numbers.

Test counts per crate at the time (`#[test]`): core 88, view 26, protocol 15,
deckio 34, cardtext 52, gamehost 219, ai 190, client-core 1176. The crates
below engine and above the client are mostly well covered; the gaps are narrow.

| # | Crate | Area | Why it matters | State |
|---|-------|------|----------------|-------|
| 1 | baylee-core | `preset::GamePreset::validate`, `RoomSetup::validate` (starting counters, free mulligans, empty deck, hand/battlefield/commander print ranges, 1000-card and print-table bounds) | Every lobby room and every record goes through it; a missed bound reaches the engine or a client's print lookup | Closed: 10 tests |
| 2 | baylee-gamehost | `record::replay` refusals (`Unbuildable`, header divergence, `Refused`, chair lines, wire spelling) | A game record is untrusted input at replay | Closed: 5 tests |
| 3 | baylee-deckio | `source::recognise` host judgement (userinfo spoof, port, id length and alphabet, dotless hosts) | A link decides what a player is told to fetch | Closed: 5 tests |
| 4 | baylee-deckio | `format` registry metadata, `ReadError`/`Skipped`/`Tally` wording and bounds | Import errors are what a player reads | Closed: 5 tests |
| 5 | baylee-view | `ManaPoolView` totals, lethal damage, `has_lost`, `is_decking_out`, `GameStatic::print` (hole vs gap), `seat_name` fallback | Clients draw rules facts from these; the print hole is a hidden-information rule | Closed: 7 tests |
| 6 | baylee-core | `ids`: generation wrap, slot bound, Display/Debug, serde as one number, `is_listed_ability`, `PrintRef::UNKNOWN` | Handle identity underlies every engine reference | Closed: 6 tests |
| 7 | baylee-core | `deckdigest` serde (`has_back_image` skipped when false, defaults) | Wire shape of `GET /decks`, no test at all | Closed: 8 tests |
| 8 | baylee-deckio | `formats/{baylee,json,yaml,moxfield}.rs` have no in-file tests (covered by `tests/formats.rs` only through whole-document round trips); `text::header`/`prefixed` edge cases | Parsing of hostile files; error paths per dialect | Closed: 31 tests in `tests/format_edges.rs` |
| 9 | baylee-gamehost | `harness.rs` (7 tests, 1030 lines), `scouting.rs` (3 tests, 319 lines): AI-seat guards that must never see hidden state | Hidden-information guarantee for AI scouting | Mostly closed: +5 scouting tests (profile-to-report disclosure, window clamp, per-seat cards, zone bound, deck intel), +6 harness (agent count, cap, trail ring, determinism). The harness loop's panic-on-offered-ability tripwire and `Repeated` halt have no direct test |
| 10 | baylee-protocol | `v1` envelope: only a handful of message kinds are round-tripped; refusal text for oversize frames lives in the gateway | Wire compatibility | Closed: 15 tests in `tests/envelope.rs` (control/engine plane, nesting, truncation, version refusals) |

`gamehost/view.rs` has 80 tests including every hidden-information guarantee
named in CLAUDE.md; no gap was found there by reading. `baylee-ai` (190, +3 `held` refit tests) and
`baylee-client-core` (1176) were surveyed only by counting: files with no in-file
tests are module roots whose tests sit in sibling directories (`lobby/`, `board/`,
…), so no gap was named there; a real survey needs the coverage tool.
