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
   - [ ] At least 175 distinct item families with tier progression and actual effects.
   - [ ] More offers, consistent item prices, automatic cross-bag/equipment fusion,
     right-click purchase-and-equip, freeze/refill and balanced income.
   - [ ] Weighted five-card loot, choose three on victory or one on defeat (explicit
     owner clarification); duplicates yield three gold. Retain printing and foil.
3. **Shared shop composition**
   - [ ] Compact title, restrained width, height-aware aligned equipment/shop frames.
   - [ ] Integrated controls and counters, individual item shelf ledges, filled empty
     slots, paginated sortable bag, gold/duel stats and compact abandon affordance.
   - [ ] Shared rarity slots/tooltips, stationary button effects, real drag ghost,
     purchase/equip/fusion and income animations with reduced-motion alternatives.
   - [ ] Original modular art and lobby tiles; no copied game assets.
4. **Deck and table integration**
   - [ ] Reuse the real deckbuilder with a run-owned card pool and free basic lands.
   - [ ] Chat initially closed everywhere. Gear tray icon and structured log-style
     panel, public opponent gear and private potions, item icons on stack/order/log.
   - [ ] Visible priority over item triggers, respecting explicit pass preferences.
5. **Rewards, endings and acceptance**
   - [ ] Shop backdrop plus loot modal and earned-gold animation; standard preview,
     rarity frames, symmetric selection, clear x/y and confirmation when complete.
   - [ ] Victory/defeat presentation, then Lobby and a genuinely fresh run.
   - [ ] Native view/input acceptance at multiple sizes, measured refresh stability,
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
