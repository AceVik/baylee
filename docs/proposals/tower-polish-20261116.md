# Tower presentation and live authority — 16 November 2026

Owner acceptance checklist for the second native playtest. This extends, rather
than erases, the measured evidence in `tower-run-redesign.md`. Unchecked work is
not shipped or validated. No production deployment is authorized.

## Integration and invariants

- [x] Review and merge Baylee `d7111b6d0` and Central `87d40e86` into the published
  feature branches, preserving the prior input/focus and idle-state fixes.
- [ ] Finish integration checks and review corrections: local stack ownership,
  strict development origins, feature matrix and production WASM reproducibility.
- Persisted cohorts retain their shop algorithms, item definitions, deck rules and
  replay semantics. New content gets an explicit version. Engine rules, prices,
  rewards and fusion are authoritative; animations never predict a committed result.
- All newly introduced helpers are Rust. Reuse the existing shell, card preview,
  deckbuilder, input, TLS and bounded transport instead of parallel implementations.
- Every iteration ends with focused checks; broad unchanged suites are excluded.

## Iterations

1. **Transport and stability**
   - [x] Bounded authenticated player WebSocket for commands, snapshots and updates.
   - [x] Transaction-commit database notifications shared across authority processes;
     one listener per process, bounded fan-out, resync on loss/reconnect.
   - [x] Fresh scope checks, owner binding, revocation/expiry, backpressure, serial
     correlation and existing mutation receipts. No credential in URL or log.
   - [x] Remove regular UI HTTP polling; unchanged data must not rebuild panels.
2. **Run flow and content**
   - [x] Three stable randomized starter offers from at least 50 meaningful decks;
     commander art/preview, shared deck inspection, immediate click-to-start.
   - [x] At least 175 distinct item families with tier progression and actual effects.
   - [x] More offers, consistent item prices, automatic cross-bag/equipment fusion,
     right-click purchase-and-equip, freeze/refill and balanced income.
   - [x] Weighted five-card loot, choose three on victory or one on defeat (explicit
     owner clarification); duplicates yield three gold. Retain printing and foil.
3. **Shared shop composition**
   - [x] Compact title, restrained width, height-aware aligned equipment/shop frames.
   - [x] Integrated controls and counters, individual item shelf ledges, filled empty
     slots, paginated sortable bag, gold/duel stats and compact abandon affordance.
   - [x] Shared rarity slots/tooltips, stationary button effects, real drag ghost,
     purchase/equip/fusion and income animations with reduced-motion alternatives.
   - [x] Original modular art and lobby tiles; no copied game assets.
4. **Deck and table integration**
   - [x] Reuse the real deckbuilder with a run-owned card pool and free basic lands.
   - [x] Chat initially closed everywhere. Gear tray icon and structured log-style
     panel, public opponent gear and private potions, item icons on stack/order/log.
   - [x] Visible priority over item triggers, respecting explicit pass preferences.
5. **Rewards, endings and acceptance**
   - [x] Shop backdrop plus loot modal and earned-gold animation; standard preview,
     rarity frames, symmetric selection, clear x/y and confirmation when complete.
   - [x] Victory/defeat presentation, then Lobby and a genuinely fresh run.
   - [x] Native view/input acceptance at multiple sizes, measured refresh stability,
     targeted integration/adversarial tests, updated PR descriptions and review.

## Transport threat review

Player clients are untrusted. The Central-issued Tower scope binds the account and
canonical audience. A socket is only a transport, never an authorization cache.
Commands keep expected revision and durable request identity; a lost connection
cannot turn an ambiguous purchase into a second purchase. Notifications carry only
invalidation hints and never grant access: every resulting snapshot is read under
the authenticated owner. A missed/coalesced hint causes a bounded resync. No public
subscription reveals another player's private inventory or potions. Concurrent
sockets, message sizes, pending frames, authentication work and dispatch have
explicit limits. UI transport state is separate from render-state equality.

### Iteration 1 evidence

- Merged Gemini branches and corrected local stack ownership/cleanup, including
  cancellation while building. Five focused supervisor tests passed.
- Isolated native PostgreSQL socket integration passed: two owner connections,
  foreign-run refusal, account connection cap, exact repeated purchase receipt,
  committed changes from a separate store writer and revoked credentials. All
  owned services were closed (`/tmp/baylee-tower-live-socket-20261116.log`).
- Envelope and core snapshot/uncertain-receipt tests passed. The native idle test
  passed and confirms no periodic HTTP job or resource-change rebuild at rest.
- Strict clippy passed on Tower API/store/service, account SDK and local dev tool.
  This is focused author QA, not full gate completion or independent review.

### Shared presentation and transaction evidence

- Four native correction passes, at 1440×900 and then 1280×720: replaced the
  activity paintings, reduced tall entry tiles, aligned shop/equipment frames,
  compacted the inventory until its complete slots and statistics fit the desktop
  floor. Final images: `baylee-tower-visual-46f0cf152405/client/02-lobby.png` and
  `05-shop.png` in the owned system temporary directory. Original asset prompts
  are kept beside the three committed landscapes.
- Shared item portrait/detail hover is used in the shop, inventory, gear drawer,
  stack and ordering tray. A pointer-following, hit-test-ignored drag ghost was
  exercised by moving a purchased equipped item back into the bag. Secondary
  purchase equips atomically; primary purchases still go to inventory. Eight
  automatic-fusion tests include cascades, full-slot refusal and rollback.
- A real native duel held the Spring Chalice upkeep trigger at Priority with a
  visible Pass control. The gear icon lives in the existing table toolbar. The
  entry path that used to reopen chat was found during this test and corrected;
  the next native run confirmed closed chat/drawer/typing at Mulligan.
- A test concession returned through “Zum Händler” to the shop-backed five-card
  modal; choosing one card and claiming it produced the next freshly rolled shop.
  This tests the retained loss flow, not the forthcoming weighted/foil reward rules.
- Focused checks: 17 native Echoes unit tests, four shared-item tests, the core
  explicit-item-priority regression, eight economy tests and strict clippy on
  client/Tower/seat. The last spacing/chat corrections were subsequently built
  and inspected natively. Idle HUD rebuild counters stayed 93→93 across the first
  inspection interval. Every owned client, backend and PostgreSQL instance was
  stopped and its PID/listener closure verified after each run.
- At that milestone still pending: 50 starters/three offers, 175 item families, additional shop
  offers, print/foil-aware weighted loot and three-gold duplicates, transactional
  animations, complete builder integration and the final run-ending scene.

### Generation five: starter draft and native passes five/six

- Fifty explicit Commander build-arounds use the shared curve/synergy builder.
  The first native inspector exposed a generic-spell problem in Rin and Seri;
  protected thematic cores now preserve 6–10 relevant cards through refinement.
  Final-deck tests independently count four tribal packages. Short mana bases
  retain a basic search target in every commander colour. Ordinary Commander
  still requires one hundred cards. These are construction checks, not measured
  win-rate calibration for all fifty matchups.
- API 7 offers three distinct stable owner-bound indices. PostgreSQL tests cover
  competing initial reads, forged starts, consent/allocation rollback, exact
  receipts, unplayed abandonment, changed-seed notifications and the real content
  publication's encoded/response bounds. Draft creation itself does not notify.
  An explicit 256-KiB immutable-content bound leaves run state at 128 KiB.
- Strict clippy passed for client, content, store and service. Focused checks
  include 88 Tower tests, 24 format regressions, 18 Echoes core tests and socket
  owner-invalidation isolation. Subsequent content corrections use the same
  fifty-deck admission test instead of rerunning unrelated suites.
- Native pass six at 1280×720 shows three equal-width choices, the shared deck
  inspector and immediate click-to-start. Normal in-flight requests no longer
  show the ambiguous-command recovery warning. Shop idle rebuild count stayed
  85→85. Evidence: `baylee-recording-01a1e54e-d690-71c3-9c70-039e429b11fd`
  (10/10 choice frames), `baylee-recording-01a1e54f-5d05-71be-8c5b-f5014a6417b8`
  (inspector) and `baylee-recording-01a1e550-80d4-774a-8f16-a5ffba68369a`
  (shop transition, 23 written and seven dropped samples). Sampling does not
  prove the absence of every transient frame. The final basic-target correction
  is covered by the content test, not by this earlier native fixture.
- Passes five and six each closed their owned client, backend and PostgreSQL;
  PID and loopback listener closure were verified. No test service remains.

## Runic catalog iteration and native passes 7–8 — 16 November

- Generation 6 adds 160 event/reward families to the original 18, with 712
  native rarity definitions in total. The new `baylee-tower-relics` crate
  shares closed identities and tuning without linking server authority into
  cards. Codegen emits static DSL definitions; `codegen --tables --check` is clean.
- Twelve saved offers include ten relics and two supplies. An exact-tier affinity
  slot improves upgrade availability without inventory order rerolling offers.
  All 178 families were reached across the deterministic sampling test. Retained
  generations keep their old families, eight offers, rules and AI identity.
- Native tests play all 16 charge events, ten reward kinds at all four tiers,
  activation costs/caps, added abilities and Expert decisions. Three single-item
  Mythic feedback recipes were tuned to consume more charges than they generate.
  Multi-item engines remain possible. Tower policy 23 values renewable charges
  separately and voluntarily stops optional token growth at an ample board;
  human actions and mandatory effects remain unrestricted.
- Ten original transparent atlases supply sixteen portraits each. Lazy shared
  materials keep image allocation bounded, and idle frames do not upload uniforms.
  The scepter atlas failed its first transparency check and was replaced by a
  clean built-in extraction edit; individual sampled cells and both shader paths
  pass. The per-cell visibility check excludes the deliberately unused apron.
- Pass 7 exposed two integration omissions: the client still required eight
  offers, and the service's draft projection recognized only generation 5.
  Both now use the shared generation contract. A client regression refuses wrong
  slot counts and unknown generations; the real HTTP/engine story validates the
  three-choice projection before starting its exact offered deck.
- Focused evidence: 92 domain cases across the domain run and focused fixture
  correction, 29 native Tower module cases across the module run and two feedback
  follow-ups, 19 client-core Echoes cases, 11 item-art cases and strict clippy on
  the changed domain/native/client/service/gateway surfaces. No full unrelated
  suite or CI wait was added. Debug linking reports macOS's large unwind-table
  warning; that is not a release performance measurement.
- A fresh isolated native integration run passed Central authentication, owner
  draft, purchase/equip, gateway routing, engine-crash recovery, independent
  verification, one settlement, five-card reward and idempotent frozen-shop refill
  in 34.99 seconds; zero quarantines. Evidence:
  `/tmp/baylee-tower-runic-native-authority-final-20261116.log`.
- Pass 8 at 1280×720 shows twelve complete shop slots and readable localized
  details. Right purchase equipped Rootbound Scepter for two gold; virtual drag
  moved it into the bag and right-click equipped it again. Buying the third
  Common across bag and equipment produced one Uncommon in the original slot.
  Six 2-second, 5-fps recordings inspected so far each wrote ten samples without
  drops; these are sampled render evidence, not every displayed frame.
- At 1600×900 the bounded panel was visibly left-aligned. The optional scrollbar
  also changes available width while a rebuilt tree is being measured. The next
  layout iteration must center the bounded workspace and reserve its gutter.
  Idle rebuild count stayed at 106 across the final interval. Purchase/fusion
  flights, weighted/printed loot, the full builder and endings remain unfinished.
- Owned native clients, backend fixtures and PostgreSQL clusters were stopped;
  exact PIDs and listeners were checked closed after both failed and successful
  runs. No fixture from these passes remains running.

## Layout and confirmed feedback, native passes 9–10 — 16 November

- Centered the bounded workspace, stretched its vertical body and reserved the
  optional scrollbar gutter from the first frame. Pass 9 inspected 1280×720 and
  1600×900; both two-second 5-fps recordings wrote all ten samples.
- Shared, non-interactive icon/coin flights now render confirmed purchases,
  equipment moves, fusion ingredients and the finished rarity glow. They live
  outside layout, retain no credentials/decks and never dispatch mutations.
  Only adjacent authoritative revisions animate; duplicate/stale replies,
  reconnect gaps and another run cannot replay spending. Reduced motion cancels
  travel. Bounded flights and binary instance lookup avoid a quadratic bag diff.
- Two renderer-free transition tests and strict client/core clippy passed.
  Native pass 10 inspected purchase/coin travel, right-click unequip/re-equip and
  a three-copy fusion spanning bag and equipment. The upgraded copy remained in
  its equipped slot. Sampled frames show convergence and the final glow.
- The first 12-fps purchase capture dropped 20/42 scheduled samples under
  readback backpressure; it is not evidence of full animation continuity.
  Repeating movement/fusion at 5 fps and slowed virtual time wrote 20/20 and
  28/28 samples. Evidence: `baylee-recording-01a1e59f-9922-72d5-814f-896743e155b0`
  and `baylee-recording-01a1e5a0-6b26-7056-8c9b-50c74a3c2a6d`.
- Both passes closed their exact owned client, backend and PostgreSQL processes.
  Weighted/printed loot, shared full builder and final ending screens still need
  implementation and their own targeted validation.

## Printed loot and ending scenes, native passes 11–12 — 16 November

- Runic rewards sample five distinct eligible cards from the verified opponent
  deck, weighted toward the player's color identity and rarer cards. Select
  three on victory and one on defeat. Owned selections yield exactly three gold;
  retained generations keep their original uniform sampling and one gold.
- The published catalog now binds a reference printing and finish to each card;
  both players' packages and the reward/commander previews reuse them. At least
  a quarter of the curated palette and all eligible commanders are foil.
  Language-specific image resolution remains unfinished.
- Focused tests cover offer provenance, deterministic weights across 512 seeds,
  claim replay, duplicate proceeds, catalog substitution, frozen deck prints and
  catalog size. Strict domain/content/client/core clippy passed.
- Two original generated landscapes use the shared lazy scene cache and frame
  shader. Native victory and defeat fixtures each presented five reward tiles,
  accepted the required 3/1 selection through the authority, then displayed the
  corresponding full run ending. Victory → Lobby → Tower offered three starters
  for a fresh run. These are synthetic outcome fixtures, not evidence that an
  independently verified native replay completed eight wins or five defeats.
- Sampled 5-fps captures wrote 10/10 reward and 12/12 victory frames, then 10/10
  selected-defeat-reward and 10/10 defeat frames, without drops. Ending evidence:
  `baylee-recording-01a1e5b1-e5bb-70a4-92ba-d5eba340e0b2` and
  `baylee-recording-01a1e5b3-b17a-76a4-9b3c-2b0ac558c6a8`.
  Owned client/backend/PostgreSQL PIDs and listeners closed after each pass.
- Round-income feedback now uses a separate once-per-visible-round latch, since
  worker lease/record revisions may be skipped while the game is visible. Three
  transition tests pass, including stale rounds and repeated pending rewards;
  native pass 13 captured the coin arc (66/75 scheduled samples written, nine
  dropped under readback backpressure, so not full continuity evidence).

## Shared editor correction passes 13–17 — 16 November

- The real deckbuilder now edits a session-local Tower collection with ordinary
  filters, list/drag code, legality display and card previews. No cloned library
  pool, second editor layout, deck-library save or persistent autosave draft.
  Main/side geometry is 40/5, one commander, shared color and singleton checks.
  Frozen prints survive adding/moving; ordinary late pool/printing replies cannot
  replace the collection. Local input remains usable during read-only requests.
- Pass 13 exposed a cold-path stall: constructing the first Tower pool row also
  built every commander partner pair. The single-commander path now shares the
  row constructor without requesting that unrelated catalogue. The failed
  fixture's watchdog closed its client/backend/database, rather than hiding the
  stall with a longer timeout.
- Passes 14–15 exposed a stale house-library page that suppressed editor patches.
  Entry now uses the normal builder's library-closing path; the existing native
  builder-entry regression test also covers scoped entry from house/history.
- Pass 16 failed in the QA helper because its quoted search hit the dev-control
  text parser's escaping limitation. The helper uses the equivalent unquoted
  exact-name search for the single-word basic-land names; no product workaround.
- Pass 17 passed actual virtual input: search for a basic of the commander's
  identity, add it to the sideboard, send the refit, wait for authoritative clean
  state, return to the shop and reopen with the persisted sideboard. Evidence:
  `/tmp/baylee-tower-builder-reopen2-native-20261116.log`. Single-frame passive
  screenshots were black and are not used as visual acceptance evidence.
- Core checks cover geometry, ownership across zones, off-color loaded cards,
  ordinary-editor isolation, fixed print inspection and refusing a full
  destination without removing the source card. Normal Commander remains 100.
  All owned fixtures from these passes exited; exact PIDs and ports were checked.

- Pass 18 repeated the persisted refit and opened the shared fixed-print card
  inspector. Both 2-second, 5-fps captures wrote ten samples without drops:
  `baylee-recording-01a1e5dd-a852-7584-96e5-847a52c86bcb` (restored sideboard),
  `baylee-recording-01a1e5dd-b5c9-73c3-8936-d3075d223b04` (inspector).
  The native path exposes no cosmetic replacement controls for run cards.
  Focused core tests (three cases), library-entry regression and strict clippy
  passed. `/tmp/baylee-tower-builder-final-native-20261116.log` records cleanup;
  all owned PIDs/ports were verified closed. No full gate claim is made here.

## Table correction pass 19 and exact-edition language palette

- The native showcase entered a real Tower duel with chat/drawer closed. Seven
  upkeep item abilities stayed at the local player's Priority until Pass; the
  stack used actual item portraits and rarity frames. The gear tray showed six
  items, private potions with inset counts and six empty opponent slots. This
  fixture had no opponent gear, so it is not native evidence for that case.
- The ordering dialog exposed missing token-ability art: it asked only for a
  printed card. The correction resolves explicit public token identities first,
  like the stack, and preserves their shared item-detail hover.
- A potion click exposed two joined defects: closing the inventory removed its
  chooser geometry, and keyboard handling rejected a one-option chooser as stale.
  The correction retains a visible item anchor and lets the existing one-option
  sheet arm/confirm normally. Native revalidation follows; the pre-fix run does
  not count as a successful potion activation.
- The ordering capture wrote 6/10 samples (four drops); the stack, gear and
  missing-chooser captures each wrote 10/10. All owned client/backend/PostgreSQL
  processes and their listeners closed (`baylee-tower-table-polish-native-20261116.log`).
- An explicit Rust maintenance fetch checked all 878 frozen printings. It found
  557 German scans of exactly the same Oracle identity, set and collector number;
  321 use the original English edition. Offline cached regeneration is byte-identical.
  The client adapter changes only presentation, preserves finish, binds table
  lookup to the current Tower game and only already-projected print identities.
  No runtime metadata request or image asset is added. Reproduction and SHA-256
  are in `assets/tower/print-languages.md`.

- Pass 20 stopped because its QA driver assumed the human started the game. The
  opponent started; the driver now passes only explicitly offered human priority
  and waits for acknowledgement until the first local upkeep. No product workaround.
  The failed fixture's exact PIDs and ports were verified closed.
- Pass 21 passed real input through ordering, manual priority, the visible
  one-option potion chooser, arming, confirmation and authoritative consumption
  of exactly one charge with healing placed on the stack. It also opened the
  item-aware log. Four 2-second, 5-fps recordings wrote 10/10 frames each, all
  visually inspected; German exact-edition cards are visible in the hand.
  Evidence: `/tmp/baylee-tower-localized-table-native-r2-20261116.log`.
  Owned PIDs 80135/80150/80201 and ports 58225/58316 closed. The final log-opacity
  refinement reduces competing stack text beneath the existing glass material;
  the shared style and single compositing layer are retained.
- Eight focused language/ordering/gear tests and three log identity/privacy
  tests passed. Strict all-target client/core Clippy and the client
  dev-control/dev-reload build checks passed. The metadata maintenance tool's
  bounded identity/path tests and byte-identical offline regeneration passed.

## Final native acceptance and author review

- Pass 22 again saved and reopened a scoped sideboard refit, then opened its
  fixed-print inspector. Both captures wrote 10/10 samples. The combined helper
  stopped before its table stage because three legitimate close surfaces share
  the same action; this is a selector ambiguity, not a product failure. The
  editor's global favourite remains an ordinary personal preference; deck-specific
  library tools remain hidden/blocked in the scoped session. Exact-edition English
  fallback was visible and intentional. Evidence:
  `/tmp/baylee-tower-final-native-20261116.log`; owned PIDs 85164/85213/87110
  and ports 62201/62887 were verified closed.
- Pass 23 independently completed the table workflow with the final log opacity:
  visible ordering icons, explicit manual priority, visible potion chooser,
  arm/confirm, one consumed charge, pending healing and the item-aware log.
  All four 2-second/5-fps recordings wrote 10/10 samples, without drops. The final
  log frame was inspected at 1280×720; German prints and English fallback coexist
  without changing earned identity or finish. Evidence:
  `/tmp/baylee-tower-final-table-native-20261116.log`; the log recording is
  `baylee-recording-01a1e604-b0f6-7724-9072-f6e38d80b8fd`. Owned PIDs
  94780/94811/94943 and ports 50084/50180 were verified closed; the backend's
  owned-listener checks also passed. No test services remain.
- The final palette change passed the existing log-contrast regression. The last
  native build succeeded. Review boundaries and corrected findings are recorded
  in `../history/tower-polish-review-20261116.md`; this is author review, not an
  independent audit. PR 427 and companion PR 67 describe the current implementation
  and keep the Central canonical WASM comparison explicitly open. Neither PR is
  made ready or merged; the owner requested a subsequent Fable review.
