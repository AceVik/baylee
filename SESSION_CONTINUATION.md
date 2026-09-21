# Baylee session continuation

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

No push is authorized. Project 6 items #195–#198 should be **In review** after
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
