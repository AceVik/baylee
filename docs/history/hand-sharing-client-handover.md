# Handover Protocol: Teammate Hand Sharing & Teamplay (Engine, AI & Client)

**Date:** 2026-11-09  
**Branch:** `c42/v6s-team`  
**Scope:** End-to-end teammate hand sharing (continuous + one-time snapshot), mode-level rules, Baylee AI teamplay & hand-sharing integration, and client preparation for the Review Agent.

---

## 1. What Is Already Implemented & Verified

### 1.1 Core, View, Engine & Gamehost (`baylee-core`, `baylee-view`, `baylee-engine`, `baylee-gamehost`)
- **Mode-Level Rule (`HouseRules::hand_sharing`)**:
  - Defined [`HandSharingRule`](file:///home/ace/Projects/baylee-league/crates/baylee-core/src/preset.rs#L94-L108) (`Allowed` [default], `OnceOnly`, `Disabled`) on [`HouseRules`](file:///home/ace/Projects/baylee-league/crates/baylee-core/src/preset.rs#L110-L150) with `#[serde(default)]` backwards compatibility.
- **Two-Tier Wire Protocol (`SeatSetting`)**:
  - Extended [`SeatSetting`](file:///home/ace/Projects/baylee-league/crates/baylee-view/src/shown_hands.rs#L28-L47) with `SeatSetting::ShareHandOnce(SeatSet)` alongside `SeatSetting::ShareHand(SeatSet)`, `SeatSetting::RequestHand(PlayerId)`, and `SeatSetting::DeclineHand(PlayerId)`.
  - Updated [`wire.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-view/src/patch/wire.rs#L1150-L1190) binary encoding/decoding (`tag 3` for `ShareHandOnce`) and round-trip tests.
- **Engine One-Time Snapshot State & View Projection (`baylee-engine`)**:
  - Added `hand_shared_once: Vec<[Option<BTreeSet<ObjectId>>; 16]>` and `Engine::show_hand_snapshot(owner, to)` in [`crates/baylee-engine/src/lib.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-engine/src/lib.rs#L685-L755).
  - Updated `build_shared_hands` in [`crates/baylee-engine/src/view.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-engine/src/view.rs#L665-L750) so a teammate granted a one-time snapshot sees only the exact `ObjectId`s that were in the owner's hand at the time of the offer (and still remain in that hand); newly drawn cards remain hidden unless shared again.
  - Added [`Engine::fork_for_with_shared`](file:///home/ace/Projects/baylee-league/crates/baylee-engine/src/fork.rs#L70-L155) and updated [`Session::fork_for`](file:///home/ace/Projects/baylee-league/crates/baylee-gamehost/src/session/views.rs#L90-L130) so lookahead forks preserve teammates' shared hands while still shuffling unshared hidden zones.
- **Session Authorization & Driven/AI Seat Access (`baylee-gamehost`)**:
  - Enforced `HandSharingRule::{Allowed, OnceOnly, Disabled}` and strict teammate-only validation in [`Session::seat_setting`](file:///home/ace/Projects/baylee-league/crates/baylee-gamehost/src/session/chairs.rs#L424-L450) and [`shared_hand.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-gamehost/src/session/shared_hand.rs).
  - Enabled `include_teammate_shared = true` for driven seats in [`Session::observe_view`](file:///home/ace/Projects/baylee-league/crates/baylee-gamehost/src/session/views.rs#L20-L88) and [`Session::view_for`](file:///home/ace/Projects/baylee-league/crates/baylee-gamehost/src/session/views.rs#L50-L88), and added [`Session::apply_team_hand_sharing`](file:///home/ace/Projects/baylee-league/crates/baylee-gamehost/src/session/chairs.rs#L110-L155).

### 1.2 Seat Bridge (`baylee-seat`)
- Added [`AiHandSharePolicy`](file:///home/ace/Projects/baylee-league/crates/baylee-seat/src/lib.rs#L55-L85) (`AlwaysContinuous` [default], `OnRequestContinuous`, `OnRequestOnce`, `Never`) and CLI flag `--hand-share-policy` in [`crates/baylee-seat/src/main.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-seat/src/main.rs).
- Wired automatic initial sharing and incoming `hand_requests` responses in [`crates/baylee-seat/src/seat.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-seat/src/seat.rs#L815-L925).

### 1.3 Baylee AI Teamplay (`baylee-train`)
- Fixed the 36 `DeclareAttackers` engine refusals caused by `Caverns of Despair` (`MaxAttackersPerCombat(2)`) and `Archangel of Tithes` (`AttackTaxMana`) in [`crates/baylee-train/src/features4/candidates.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-train/src/features4/candidates.rs#L769-L870), [`attack_subset.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-train/src/features4/attack_subset.rs), [`sampling.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-train/src/features4/sampling.rs), and [`targets.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-train/src/features4/targets.rs).
- Added zero-allocation thread-local team context (`with_team_context`, `is_ally`, `is_opponent`) in [`crates/baylee-train/src/netplay4/tactics.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-train/src/netplay4/tactics.rs#L23-L75) and updated friend/foe checks across `tactics.rs`, `choices.rs`, `tactics/guard.rs`, and `tactics/rules/{read,options,trick,ping,targets,attack}.rs`, plus team board-wipe protection, anti-double-counterspell logic, Focus-Fire multi-defender targeting, and team-aware lookahead rollouts in [`rollout.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-train/src/netplay4/search/rollout.rs).

### 1.4 Client Brain & Transport (`baylee-client-core` & `baylee-client`)
- **Headless State & Action Module**: [`crates/baylee-client-core/src/handshare.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-league/crates/baylee-client-core/src/handshare.rs) (exported as `baylee_client_core::handshare`):
  - `TeammateHandStatus<'a>` (`player`, `sharing_to_them`, `requested_by_them`, `visible_from_them`)
  - `teammates_of(view, teams)`, `teammates_of_seat(view, actor, teams)`, `is_teammate(view, teams, a, b)`
  - `teammate_statuses(view, teams)`, `incoming_requests(view, teams)`, `shared_hand_of(view, player)`
  - Pure `SeatSetting` constructors: `share_continuously`, `stop_sharing`, `toggle_continuous_share`, `share_all_teammates_continuously`, `share_once`, `share_all_teammates_once`, `request_hand`, `decline_request`
  - Localized label helpers: `label_share_continuous`, `label_share_once`, `label_stop_sharing`, `label_request_hand`, `label_decline_request`, `label_requested_by`
- **Localization (`locales/en/` & `locales/de/`)**:
  - Added `HandShareWithTeammate`, `HandShareOnceWithTeammate`, `HandStopSharingWithTeammate`, `HandRequestFromTeammate`, `HandDeclineRequestFromTeammate`, `HandRequestedByTeammate`, `HandSharingActiveWithTeammate`, `HandShareAllContinuous`, and `HandShareAllOnce` in [`locales/en/interface.ftl`](file:///home/ace/Projects/baylee-league/crates/baylee-client-core/locales/en/interface.ftl) and [`locales/de/interface.ftl`](file:///home/ace/Projects/baylee-league/crates/baylee-client-core/locales/de/interface.ftl).
  - Added all 5 hand-sharing server refusal translations in [`locales/en/server.ftl`](file:///home/ace/Projects/baylee-league/crates/baylee-client-core/locales/en/server.ftl) and [`locales/de/server.ftl`](file:///home/ace/Projects/baylee-league/crates/baylee-client-core/locales/de/server.ftl).
- **Client Host & Outbox Plumbing (`baylee-client`)**:
  - Added `DuelHost::seat_setting(&mut self, setting: SeatSetting)` in [`crates/baylee-client/src/host.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-client/src/host.rs#L116-L121) and implemented it in `LocalHost`, [`BackgroundHost`](file:///home/ace/Projects/baylee-league/crates/baylee-client/src/host/background.rs#L140-L152), and [`NetworkHost`](file:///home/ace/Projects/baylee-league/crates/baylee-client/src/net.rs#L762-L772).
  - Added `Duel::submit_seat_setting(&mut self, setting: SeatSetting)` in [`crates/baylee-client/src/duel.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-client/src/duel.rs#L1005-L1023) and flushed `seat_settings_outbox` in [`flush_outbox`](file:///home/ace/Projects/baylee-league/crates/baylee-client/src/lib.rs#L1564-L1583) without clearing `duel.interaction`.
- **Zone Browser & Previews (Already Wired)**:
  - [`BrowseZone::ShownHand(player)`](file:///home/ace/Projects/baylee-league/crates/baylee-client-core/src/browser.rs#L121-L125) and [`browser/shown_hands.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-client-core/src/browser/shown_hands.rs) automatically expose a tab in the Zone Browser for any teammate with cards in `view.shared_hands`.
  - [`decision::known_cards`](file:///home/ace/Projects/baylee-league/crates/baylee-client-core/src/decision.rs#L61-L67) and [`social/window/preview.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-client/src/social/window/preview.rs#L38-L41) already resolve card identities from `view.shared_hands`.

---

## 2. Remaining Work for the Review Agent in `baylee-client`

### Task 1: Teammate Hand Controls in the Table UI (`baylee-client/src/hud/`)
Wire the functions in [`baylee_client_core::handshare`](file:///home/ace/Projects/baylee-league/crates/baylee-client-core/src/handshare.rs) into the table HUD when the table has teams (`duel.statics.as_ref().map(|s| s.teams.as_slice())`):
1. **Clicking a Teammate's Hand Count Cell / Plate**:
   - In [`crates/baylee-client/src/hud/seatbar/attached.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-client/src/hud/seatbar/attached.rs) and/or [`crates/baylee-client/src/hud/ledge/players.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-client/src/hud/ledge/players.rs):
     - If `handshare::shared_hand_of(view, teammate)` is `Some(cards)` and non-empty, clicking the teammate's hand count opens `duel.browser.open_at(BrowseZone::ShownHand(teammate))`.
     - If `handshare::shared_hand_of(view, teammate)` is `None` and `handshare::is_teammate(view, teams, view.seat, teammate)`, clicking or using the teammate menu can send `handshare::request_hand(view, teams, teammate)` via `duel.submit_seat_setting(setting)`.
2. **Game Menu / Teammate Sharing Controls**:
   - In [`crates/baylee-client/src/hud/ledge/menu.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-client/src/hud/ledge/menu.rs) (or a small popover on the local player's hand / seat plate when `teammates_of(view, teams)` is non-empty):
     - Offer per-teammate or all-teammate actions using `handshare::teammate_statuses(view, teams)`:
       - `handshare::toggle_continuous_share(view, teams, teammate)` (`label_share_continuous` / `label_stop_sharing`)
       - `handshare::share_once(view, teams, teammate)` (`label_share_once`)
       - `handshare::request_hand(view, teams, teammate)` (`label_request_hand`)
     - Dispatch the chosen `SeatSetting` through `duel.submit_seat_setting(setting)`.

### Task 2: Incoming Hand-Request Banner / Strip (`view.hand_requests`)
- When `handshare::incoming_requests(view, teams)` is non-empty, render a non-blocking notification strip/pill on the HUD (similar to `lostbanner` or `reveals`, never blocking `duel.interaction`):
  - Show `handshare::label_requested_by(lang, &teammate_name)`.
  - Provide three action buttons for each requesting teammate `requester`:
    1. **Share (whole game)**: `handshare::share_continuously(view, teams, requester)` -> `duel.submit_seat_setting(setting)`
    2. **Show (once)**: `handshare::share_once(view, teams, requester)` -> `duel.submit_seat_setting(setting)`
    3. **Decline**: `handshare::decline_request(view, requester)` -> `duel.submit_seat_setting(setting)`

### Task 3: Room Rules UI for `HouseRules::hand_sharing` (Optional)
- In the room rules configuration UI (`crates/baylee-client/src/lobby/`), when teams are enabled, allow the host to toggle `HouseRules::hand_sharing` between `HandSharingRule::Allowed` (default), `HandSharingRule::OnceOnly`, and `HandSharingRule::Disabled`.

---

## 3. Review Checklist for the Review Agent
1. **Security & Hidden Information (CR 400.2)**:
   - Verify that [`Session::seat_setting`](file:///home/ace/Projects/baylee-league/crates/baylee-gamehost/src/session/chairs.rs#L424-L450) and [`shared_hand.rs`](file:///home/ace/Projects/baylee-league/crates/baylee-gamehost/src/session/shared_hand.rs) reject any attempt to share a hand with an opponent, oneself, or an out-of-range seat, and respect `HandSharingRule::{Allowed, OnceOnly, Disabled}`.
   - Verify that `SeatSetting::ShareHandOnce` only exposes the `ObjectId`s present in `owner`'s hand at the time of the snapshot and never leaks cards drawn later.
   - Verify that `Engine::fork_for_with_shared` only preserves hands that are genuinely shared with the forking viewer (`public || shared.contains(viewer) || snapshot.contains(id)`).
2. **Determinism & Performance**:
   - Verify that `SeatSetting` never mutates the engine's deterministic game-state hash (`Engine::snapshot_hash`) or journal.
   - Verify that `with_team_context` in `crates/baylee-train/src/netplay4/tactics.rs` uses stack/thread-local `Cell<[Option<u8>; 8]>` with RAII restoration (`TeamContextGuard`) and zero heap allocations.
3. **Tests to Run**:
   - `cargo test -p baylee-gamehost shared_hand`
   - `cargo test -p baylee-seat --lib`
   - `cargo test -p baylee-train --lib`
   - `cargo test -p baylee-client-core --lib handshare`
   - `cargo test -p baylee-client-core --lib i18n`
   - `cargo test -p baylee-client --lib`
