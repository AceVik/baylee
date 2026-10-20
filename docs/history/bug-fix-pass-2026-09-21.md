# Client and engine bug pass — 2026-09-21

This is a local implementation and verification record. GitHub issues were not
closed automatically. The board and issue-open state disagree on several tickets;
a board row marked Done is not evidence that every acceptance criterion passed.

## Changes in this pass

| Ticket | Change and regression coverage |
|---|---|
| #182 | Player-only selections now invalidate the action bar and show Confirm. Pointer tests check the submitted player; a headless HUD test checks Confirm appears and disappears. Halimar Excavator already has an engine test proving its selected player mills. |
| #112 | Refused hand-card clicks distinguish missing targets, timing and unavailable costs in English and German. The existing touch refusal feedback remains in use. Tests cover all three reasons. |
| #130 | Fresh accounts default to Every Step. Upkeep and draw are no longer skipped by the default rail preset. Explicit saved settings remain respected. |
| #38 | Off-board projection is scoped per effect; an unrelated cross-zone effect no longer spreads battlefield-only effects into libraries. |
| #120 | The refresh after the last cross-zone effect disappears also revisits off-board objects. The Maskwood Nexus library test now removes the Nexus and checks that Ally disappears. |
| #118 | Flashback replaces stack departures with exile, including countering and bouncing. Permanent-casting permissions such as Emry no longer receive the flashback marker. Tests cover three destination zones, normal spell controls, and Emry's existing game regression. |
| #122 | Effect identity hashes include effect id, source generation, filter, object version and all modifier payloads. Tests distinguish targets, target versions and granted costs; identical clones remain equal. This does not complete the broader #86 audit. |
| #158 | Plain mana retains its snow-source provenance; colored snow mana pays snow costs and ordinary colorless mana does not. Basic-land and effect production preserve provenance. Wild payment preserves snow requirements. Mouth of Ronom's game test now pays with a Snow-Covered Forest. |
| #172 | Hybrid and two-or-color assignment backtracks; probe and payment share one solver, and failed payment leaves the pool unchanged. Tests cover both reported failures and small-pool agreement. |
| #173 | The targeting test helper returns at quiet priority and fails on unexpected questions or exhaustion. A regression checks that already-quiet priority is unchanged. |
| #175 | The text-span guard requires spans in each named source, with a negative test for one empty source. |
| #180 | A rejected in-process AI action invokes the general timeout-answer policy instead of silently returning. If that also fails, the game process fails explicitly. The regression injects an illegal action during mulligan and checks progress. |
| #141 | Corrected the stale client documentation: land drops and abilities costing only their own tap are existing one-click exemptions. The existing behavior and its gesture tests are retained. |

## Partially addressed

- #140: player-target confirmation and aim-following are fixed. Candidate lighting,
  the initial aim indication, and graveyard-discovery guidance still need review.
- #128: fresh accounts skip empty attacker/blocker declarations. Explanations for
  individual illegal combat candidates remain outstanding.

## Existing fixes found in the checkout

These issues already had relevant code and tests before this pass; they were not
reimplemented or automatically closed: #9, #74, #87, #109, #114, #119, #123,
#139, #148, #149, #150, #151, #155, #157, #159, #165, #166, #167 and #170.
The workspace suite checks their existing regressions; this is not a claim that
all prose acceptance criteria have been independently reproduced.

## Still outstanding

- #1, #2: Android emulator startup and iOS simulator window sizing.
- #83: presentation of losses caused by the clock.
- #86: complete state/driver snapshot-hash audit and missing continuation fields.
- #131: preview overlap and target accessibility across window sizes.
- #132: full details for every stack entry (word-boundary truncation already has a fix).
- #133: consistent card-face and creature-type localization.
- #137: remaining sign-in and lobby presentation problems.
- #142: overload/modal selection before floating the chosen cost.
- #143: reproduce both reported silent trigger outcomes and explain resolution failures.
- #144: initial clock cue, changing mulligan budgets and stable button positions.
- #145: combat wording, shockland identification, pile images, end-state counters and card sizing.
- #152: propagate modal/transforming face permissions through codegen, engine and AI.
- #179: add the created-effect half of the granted-mana offer/view consistency guard.

Tooling/catalog tickets #171 and #174 are outside the requested client/engine fixes.

## Commits and validation

- `1bf9366d`: engine fixes (#38 #118 #120 #122 #158 #172 #173).
- `239157e0`: AI recovery (#180).
- `98e1d8a4`: client fixes (#182 #112 #130; partial #140 #128; guard #175).
- `81991e1e`: interaction documentation (#141).

Validation completed locally:

- `cargo test --workspace --all-targets`: **3,673 passed, 0 failed, 3 ignored**, with the local test database and local server access enabled.
- Final client library suite, including the additional refusal regression: **755 passed, 0 failed, 1 ignored**.
- Engine library suite: **1,088 passed**; the subsequently strengthened Mouth of Ronom test also passed.
- Game-host library suite: **80 passed, 1 ignored**, including the invalid-AI-action recovery regression.
- Clippy with `--all-targets -- -D warnings` passed for core, engine, client-core, client and gamehost.
- Formatting and whitespace checks passed.

The client test linker reports its existing compact-unwind size warning. No UI
screenshots or emulator runs were used to claim mobile or layout fixes. These
commits remain local; no existing issue was closed and no push was performed.
