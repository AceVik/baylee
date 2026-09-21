# Baylee session continuation

## Stack automation and localization (2026-09-21)

- User explicitly chose to stop **before** the marked ability resolves.
  Stack entry clicks mark that boundary; legal target choices take precedence.
  Resolve-to-selection uses UntilTopOfStack; both engine standing yields and
  client phase automation respect the reached boundary until manual priority.
  Intervening optional answers cannot clear the stop; cancellation also pauses.
- Added per-AbilityRef standing yields, independent optional Yes/No policies,
  atomic policy updates and deterministic snapshot coverage. Defaults remain
  compatible with older preferences. Policies persist locally/account-wide,
  are installed in the engine, and can be reset individually or together.
  Targets and payments stay manual.
- Replaced the seven-entry stack cutoff with a bounded virtual scrolling
  window and height-preserving spacers. Hover, selection and language changes
  retain scroll position; 2,000 entries tested in the native Metal renderer
  and headless retained-HUD tests. Test fixtures/screenshots stay in /tmp.
- Added English/German stack controls, translated finish names and known
  engine/gateway refusals. Language changes invalidate the HUD immediately.
  Unknown technical diagnostics retain their original text; card translations
  still depend on availability and retain the existing English fallback.
- Fixed the engine automation safety cap: it now returns a usable unanswered
  choice instead of a stale decision already consumed by auto-answering.
  Covered with 1,800 additional triggered abilities.


Validation: the full workspace/all-targets run passed 3,752 tests (three existing
ignored). Subsequent atomic-policy and stop-guard refinements were rechecked in
the affected crates: final targeted runs passed 794 client tests (one existing ignored),
843 client-core tests and 1,107 engine tests. The final HUD-only rerun passed
203 tests. Workspace clippy (`--all-targets -- -D warnings`), wasm32 client
check, formatting and diff checks passed. Native review covered scrolling and
marking with 2,000 entries and both English/German labels. Fifteen new regression
tests cover the stack, automation, persistence and localization changes.

## Engine audit (2026-09-21)

- Zero-loyalty planeswalkers now leave the battlefield even when they are
  also indestructible creatures (CR 704.5i).
- Combat offers and validation exclude phased-out attackers, blockers and
  planeswalkers. Resolving PhaseOut removes the permanent from combat;
  a departed blocker still leaves its attacker blocked, with trample handled
  separately. Phased-out attacked planeswalkers receive no combat damage
  and produce no lifelink gain (CR 702.26b).
- Battlefield SBAs skip phased-out permanents for lethal damage, loyalty,
  legend choices, counter annihilation and attachment cleanup. The deathtouch
  damage window still closes at each SBA check, including while phased out.
- Ordered zone removal fast-paths the last entry with pop, including the
  projectable spell subset. Middle removals still preserve order and replay
  determinism. This avoids quadratic zone-removal work when draining a stack.
- Added 11 tests: nine reproduced failures before their fixes, one guards the
  deathtouch/phasing boundary, and one checks mixed removals against an ordered
  reference list in all eight zones. Two Criterion stack-drain benchmarks
  and measured results are in docs/perf-baseline.md (20k: 63.76 ms → 45.26 µs,
  zone-removal microbenchmark only).

Validation: `cargo test -p baylee-engine --all-targets` passed 1,103 tests
plus Criterion smoke checks. `cargo test --workspace --all-targets
--no-fail-fast` passed 3,739 tests (3 existing ignored tests), with DATABASE_URL
set to the local compose PostgreSQL instance. The initial workspace attempt
stopped because DATABASE_URL was unset; all 15 catalog integration tests passed
once configured. Workspace clippy with `-D warnings`, formatting and diff
checks passed. The previously stopped PostgreSQL service was started for this
validation and stopped again afterwards.

Rule references: official Comprehensive Rules, 2026-06-19,
https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf,
CR 702.26b, 704.5h–j and 704.5q. This audit covers the listed combat/SBA
paths, not complete support for every phasing interaction.

## Client audit and push

The subsequent client audit fixed these additional failures:

- WebSocket errors without a Closed event now enter the reconnect path, on
  the initial connection as well as established and redialled connections.
- Failed card images explicitly reload their existing asset handle. Materials
  retaining a handle no longer prevent retries, and successful retries reach
  the cache observer. Loading/error overlays reset when art changes or retries.
- Hover previews move between separate rows showing identical art, ignore
  unrelated delayed Out events, and reposition when the window shrinks.
- Nested controls and scrolling no longer stop working at arbitrary ancestry
  depths. The nearest matching ancestor still owns the action.
- Background search suggestions and the mobile text input cannot remain active
  over the artwork dialog. Opening it invalidates pending clipboard reads;
  closing it does not unexpectedly reopen the phone keyboard.
- Refreshing printing metadata repairs incompatible language/set filters instead
  of leaving the carousel empty.

Validation: client and client-core `--all-targets` suites passed 1,676 tests
(1 ignored), including real local WebSocket connections, reconnection, complete
combat paths, and a simulated image that fails once before loading successfully.
Workspace clippy with `-D warnings` and the wasm compilation also passed.
The user requested pushing this audited state, including the previous local
milestones, to `origin/main`. Preserve the unrelated untracked files below.

## Follow-up fixes: combat, zone browser and artwork selection

- Combat confirmation now rebuilds when attacker/blocker assignments change,
  without requiring another game snapshot.
- Zone lists retain their scroll offset on snapshot/selection/relayout changes.
  Image arrivals no longer rebuild the dialog, preventing flicker and scroll reset.
- Larger artwork carousel and arrows; matching square refresh/close controls;
  multilingual set-code/name autocomplete (all sets on focus), keyboard navigation,
  and the finish override checkbox aligned with the finish buttons.
- Refresh and pending images show animated loading indicators. Failed images
  stop spinning; loading overlays remain attached until their image arrives.
- Shift flips artwork-picker and deck-builder previews, including ordinary backs.
  Catalog/gateway printing metadata now includes layout for true double-faced art.
- Improved etched engraving and three cosmetic styles: Holographic, Glitter,
  Galaxy. All share the board/UI shader and persist through deck import/export.
  Cosmetic styles are available with the finish override enabled.
- Public catalog pagination now spaces requests, retries rate limits using
  Retry-After, and retains successfully fetched editions if a later page fails.
  Live investigation reproduced HTTP 429 on page 21 for Forest; previously that
  discarded all 3,500 editions already loaded.
- VIEW_VERSION is now 27 for the new finish enum variants; deploy matching
  client/server versions together.

Validation for this follow-up:

- Client/core/engine-host/view library suites: 1,797 passed, 2 ignored;
  catalog wire/schema tests: 25 passed; gateway pool tests: 12 passed.
- Workspace compile check, strict clippy, native dev-control build, wasm check.
- Live Metal: all cosmetic styles, Shift flip, autocomplete with real multilingual
  editions, refresh loading indicators, and Galaxy in hand/preview/on the board.
  The final Forest fetch completed with all 3,983 editions after rate-limit handling.
- Regression tests cover combat buttons, retained zone scroll, loading-overlay
  lifetime, multilingual set lookup and cosmetic-finish save/reopen/export.
- Test configuration remains isolated under `/tmp/baylee-finish-check`.

## Completed in the September 21 continuation

The previously uncommitted #195–#198 deck-editor refinements are included in
this milestone, together with the follow-up printing and rendering work:

- Shared animated, artwork-aware foil and etched shaders with rounded edge
  highlights. Reduced motion freezes the animation.
- Finished deck thumbnails use the same material as the picker and previews.
- Fixed cross-game art reuse: game-local print references no longer retain
  textures, preloads or UI/board materials from the preceding game.
- Printing picker: five-image comparison strip, set-name and language filters,
  arrow-key navigation, refresh, and an explicit enforce-foil/etched checkbox.
- Refresh bypasses stale gateway metadata, preserves selection and keeps old
  metadata when the request fails. Multilingual/variant Scryfall results are
  requested; the missing multilingual flag caused sparse German choices.
- Reopening an ordinary deck row selects its reference art instead of the
  newest promo. Forced finishes survive deck-row save/reopen.

## Validation

- Full client/core library tests: 777 client + 837 core passed (1 ignored),
  including WGSL validation and regression tests
  for game caches, finish overrides, refresh and printing selection.
- Strict client/core clippy, native dev-control build and wasm compile check.
- Live Metal GPU checks: multilingual picker, checkbox, deck thumbnails, foil
  and etched hands and hover previews, played foil land, and game transitions
  with different decks. No GPU shader errors were observed.
- Live game verification used an isolated `/tmp/baylee-finish-check` config;
  the user's saved decks were not edited. Temporary screenshots and logs live
  under `/tmp/baylee-*` and are not repository deliverables.

## Follow-up

Project 6 items #195–#198 should be **In review** after
this milestone. Preserve the pre-existing untracked `.junie/` and
`gateway-store.json.imported`; neither belongs in this commit.

Useful checks:

```sh
cargo test -p baylee-client -p baylee-client-core --lib
cargo clippy -p baylee-client -p baylee-client-core --all-targets -- -D warnings
cargo check -p baylee-client --target wasm32-unknown-unknown
cargo build -p baylee-client --features dev-control
git diff --check
```
