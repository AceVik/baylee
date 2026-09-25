# baylee-client

`docs/client.md` is normative (a bare §"…" is its section). Loads only under `crates/baylee-client/`; open by hand for `baylee-client-core` work.
The narrative version this replaced is `docs/history/baylee-client-CLAUDE-2026-09-24.md`.

## Hosts and connection

- The renderer has no socket, only a `DuelHost`: `LocalHost` (in-process engine vs house AI) or `NetworkHost` (`src/net.rs`, `/games/{id}/ws`), same envelopes.
- `standalone::run` (`src/standalone.rs`; `main.rs`, `android_main`) picks `NetworkHost` iff given a `SeatTicket` (`BAYLEE_GAME` + `BAYLEE_SEAT_TOKEN`, or `?game=…&token=…`, gateway `?gateway=…` kept in localStorage `baylee:gateway`); else `LobbyPlugin`.
- `DuelHost::link()` → `LinkState` (Local/Up/Connecting/Down/Refused); `keep_the_table_connected` redials (`Connecting` distinct, else every frame), asking for missed frames, and never after `Refused` (the gateway's `HelloAck{compatible:false}`, #271: a hard stop naming which side is behind). `InstalledHost` is `Box<dyn DuelHost>`: link policy goes via `link()`.
- `Retry` (`baylee-client-core/src/reconnect.rs`): 0.5 s doubling to 15 s, 12 dials, then an announced give-up.
- Socketless seat: decision clock stops, reconnect clock runs; at expiry the house AI takes over (zero window: never). `GameStatic::reconnect_secs` carries it; `reconnect::Window` keeps never-told/forever/secs distinct.
- The banner turns at `Retry::brief` (`PATIENCE` 8 s or the window, earlier; never if unknown or forever), in future tense (the handover is unobservable; §"What the banner may claim…").
- Give-up is `DuelReport::Unreachable`, never `Failed(String)` (single-action refusals; never ejects).
- `PlayerView::decision_remaining_ms` is relative; `DecisionClock` (`baylee-client-core/src/decisionclock.rs`) counts down between views, each correcting it. Shown from 60 s; `Cue::ClockLow` at 60 and 10 s, latched per question, own seat only. `LedgeRevision` holds its presence, never its value (else rebuilds every second).

## Tests

- A `Duel` literal proves nothing about wire-filled fields with a legal empty value (`reachable: {}`, `owed_plan: None`); fill via `Duel::receive_view`, `receive_choice`, `rebuild_board`, `poll_host` (`owed_tests::seat_with_two_forests`). Fine where set on purpose or written by the system under test (`link_note`, `cues`, `browser`).
- Test the join, not only each end (`table::placements` → `duel.proposing()` → `owed_plan`; `table::offer_tests`).
- Run systems in a harness (`combatlines::running`), assert outcomes, prove it by removing a resource.
- `camera_tests` project pod corners forwards; never reuse the inverse under test.
- Keep card-quad mesh tests; transform tests miss bad meshes. Brightness bounds are two-sided.
- `tests/duel_flow.rs::a_whole_game_can_be_won_through_the_clients_combat_path` reaches `GameOver`, every decision built by `Interaction`.

## Lobby and rooms

- `LobbyPlugin` (`src/lobby.rs`): login, `POST /decks`, create/join, `SeatTicket` → `NetworkHost`. Decisions stay headless in `baylee-client-core/src/lobby.rs` (→ `LobbyRequest`).
- Only in `DuelPhase::Closed`, own 2D camera and plugin, so `DuelPlugin` stays embeddable.
- "Add the starter deck" posts `data/acceptance-decks.txt`'s Allytifact rows; "play the house AI offline" is `LocalHost`, no account.
- `POST /lobby/games {deck_id, mode, seats: 2..=8, name, password, clock?, decision_timeout_secs?, reconnect_window_secs?}` (8: `GamePreset::validate` = gateway `MAX_SEATS`); `.../seats/{seat}` sets kind, ai, deck, team; `.../leave` frees a chair.
- Host sets `kind`/`ai` on unoccupied chairs only; players set `deck_id` on their own only.
- Each player `POST .../ready {ready}` (409 deckless; cleared if the host swaps it), then host `.../start` (409 until all ready; configured AI chairs are).
- `POST .../host {seat}` hands the room on; leaving passes it to the earliest joiner; empty, it closes. Passwords list only as "locked".
- Players are handles `Name#tag` (tag: `account.tag`, lowercase hex, padded to 4, may grow; `baylee-gateway/src/handle.rs`), never account ids; `you`/`yours` = "is that me", `startable` = would Start work. Only `store::display_names` joins name and tag; nothing below the gateway knows tags.
- `GET /lobby/games` → `{games,total,offset,limit}`, `q` over table and host, fixed total order (waiting, newest, id): games sit in a `HashMap`.
- `GET /lobby/ws?token=…&q=…&offset=…&limit=…` pushes the page on change; search/pager re-dial. `src/lobby/feed.rs` holds it; the 2 s HTTP re-read waits behind `Feed::live()`.
- `mode:"ai"` and join order an engine (seat socket waits 30 s); an open table orders none: dial the seat once the feed says `"playing"`.
- Only the lobby is responsive: `Metrics::of(width)` (`src/lobby/ui.rs`) picks phone/tablet/desktop, sets every size; phone: stacked panels, no gateway line, 44 px targets.
- On wasm, `softkeys.rs` keeps one invisible but focusable `<input>` (focus raises the phone keyboard; autofill, IME, paste); own key handling is off there (no double input).

## Deck builder

- `Screen::Build`; decisions in client-core `deckbuilder.rs` + `deckbuilder/`.
- Pool: `GET /pool` (registry, `Coverage`), never the catalog; fetched once a session and forgotten at sign-out (`DeckBuilder::forget_pool`), filtered locally. "Playable only" hides stubs, marks Partial. One row per card; search covers `alt_names`.
- `DeckBuilder::problems` mirrors `POST /decks`: blocking greys Save; advisory (60 cards, 15 sideboard, lands, unimplemented) never blocks.
- Sideboard: a real list through store, `DeckBody`, `LoadedDeck`, `SeatSpec`.
- `open_picker`: `GET /printings` → `deckrow::PrintChoice`. `Entry` is `(slot, count, print, note)`: two printings, two rows; copy limit per card; the uneditable note survives every save. A no-op choice writes nothing (default printing leaves `4 Lightning Bolt`).
- `?` menu: add, move deck↔sideboard (keeping the printing), remove, set commander.
- Hover preview is outside the retained tree (`Hovered` → `CardPreview`, epoch, `src/lobby/preview.rs`).
- Pips: `baylee-client-core/src/manapip.rs` decides, `src/manaui.rs` draws; the Mana font gives a monochrome mark, the disc is ours; hybrid: one disc, two half-clipped glyphs (docs/legal.md §2).

## The table

- A row that can't fan until a third of each card shows scrolls sideways (`Duel::rows`, `rowbar.rs`). Cards outside its window are `Visibility::Hidden`, never despawned.
- A battlefield row stands in sections (`client-core::board::Section`: basics and tokens left, legends and utility lands right), decided by what a card is, never by what it is doing. Identical permanents pile on any row, keyed on every visible difference (`PublicObject::summary_key` plus `Proposal`, a mana plan's taps included; `a_counter_is_never_merged_away`). A pile's at most five slabs step left, and the row holds their room (`layout::pile_reach`). `docs/client.md` §"A row stands in sections".
- An aura, equipment or fortification lies under its host (#305, `CardGroup::attached`, `table::tuck`), whoever controls it (CR 303.4e, 301.5d), peeking forward by the measured room to the ledge band or the next row, turned with a tapped host and above Defender's wall (`ATTACH_DEPTH`); a pile holds its cell before a host (`Lane::gaps`); `tuck_tests`.
- Unlit: no lights, `Tonemapping::None`.
- Never position directly: `table::sync_scene` sets a `Motion` target, `table::glide` moves it (`1-e^(-rate·dt)`); `ShownRig` likewise for camera (yaw the short way); `reduce_motion` disables both.
- Generated, no sprites or textures. `tabletop.rs` CPU references: fixed-seed value-noise, no rand, no clock; the felt shader takes one OS-random seed per duel (`feltmat::TablePattern`, `host::fresh_seed`), stable across updates, resizes, reconnects (docs/legal.md §2).
- `felt.wgsl` draws the slab, `mat.wgsl` each mat, `tabletop.rs` bakes mat glow (RGBA8). `tabletop::felt`/`seat_mat` mirror the WGSL: change both (`the_shader_and_the_generator_agree_about_the_{cloth,mat}`).
- `rounded_slab_mesh` builds table and card (face, wall borrowing its UVs, contact shadow). Table hangs below the placement plane; `APRON` stays darker than the rail.
- Slab: racetrack of layout+`SLAB_MARGIN`, framed at layout+`AIR`; keep the corners (`table_corner` 0.065), the only sky. `RAIL_WIDTH` 0.55.
- An altar, not a tray: a crest of light at the cloth's edge; the rail falls `RAIL_LIP`→`RAIL_HIDE` on a quarter cosine.
- `under_lamp`/`under_sky` multiply table colour; the lamp darkens the ends. `the_felt_is_dark_enough_to_read_cards_against` bounds cloth both ways.
- Firewheel: `client_core::firewheel` (§"The table itself"), flat in `felt.wgsl`, lit after the sky, black body a multiply lit on its rim, rings first.
- Sky: `baylee-client-core/src/sky.rs` decides (`SkyMode`, pure `phase(mode,hour)`), `src/sky.rs` paints a screen-space quad (`sky.wgsl`); `sync_sky` → `table_light` → `under_sky`. UTC hour via `web-time`, never `SystemTime::now` (wasm panic). The tint stops at the phase lamp: emitted light carries meaning.
- CR 731 day/night overrides the ambient `SkyMode` (`sky::sky_target`, `RULES_FADE_RATE` 2.5 vs `FADE_RATE` 0.55); seat-bar hinges show it too.
- Mat: from `SeatSlot`, three lanes, rim gilt for the viewer else pie order, opacity = `Mood`. Turn rim reads `Mood.on_turn`, not `Standing` (`TURN_BASE`, swell 11 s, pulse 7 s).
- `SeatRole::Away` dashes the rim (`rim_dash`, mean gain 1), never dims (§"The rim says it…").
- `MAT_MARGIN` only in `client-core::tabletop`. `LEDGE_FRAC` + `COMBAT_FRAC` + 3·`LANE_FRAC` + `MARGIN_FRAC` = 1 (const assert); `ledge_corners` returns the ledge band; `the_mat_fences_its_bands_where_the_layout_put_them` reads the texture, `/state.shelves` vs screenshot live.
- From three seats every seat gets `layout::standard_board`, a duel's width on the same canvas (#264); `MAX_RING_X`/`MAX_RING_Y` 92/35 seat eight at it, `CameraRig::MAX_DISTANCE` 300; the round ring is offered at three only. `MAT_LEDGE` (1.00) is bounded by `< 0.75·CARD_HEIGHT` (1.048), no longer by the ring ceiling.
- `OwnBoardOverlay` is gone on purpose: no flat second own board, no full-canvas opaque panel over the table.

## Camera and layout

- Never hard-code the rig: `CameraRig::home(layout, canvas)` from `TableLayout::extent` and `Canvas` (window minus bottom hand zone; `Canvas::hud.top` = 0); `frame_table` reapplies on seat, focus, window change, pausing on a single-seat view. The fit is one division.
- `CAMERA_LEAN` (0.36, owner's call) is the ring lean, wide duels ease to `DUEL_LEAN` (0.62); it trades against equal board widths (`every_seat_is_drawn_a_board_of_the_same_width`); FOV cannot help.
- Three-seat FFA sits on a circle (`layout::ROUND_COST`).
- No orbit, zoom or pan (no `input::camera_controls`). Viewpoints via keymap (F, H; docs/keyboard-map.md).
- Place UI via `table::Lens` from `CameraRig::eye`, never the camera's `GlobalTransform` (a frame stale).

## Seat bars

- Bars sit on the centre-side `MAT_LEDGE` shelf (`LEDGE_IS_OUTER=false`). `MatParams::ledge_outer` is a flag, never a flipped `uv.y`. No card on the band (`no_card_reaches_the_band_its_seat_writes_on`).
- No seat's ink is drawn under the hand zone (#303, `seatbar::attached::under_the_hand`): bars sit at `GlobalZIndex(-1)` under a see-through skirt, so `attached::place` hides a panel the hand zone would partly cover.
- Caret, colour, name, life, four zone counts, turn with day/night, twelve steps; `attached.rs` poses three panels by one `pose_on` and scale.
- Own retained tree on `hud::BarRevision`, never `HudRevision`; rotated via `UiTransform::from_rotation`.
- `Density::for_length`: Full→Compact→Pip→Mark; the hover sheet carries the dropped. `for_shelf` picks `Split` on a long, deep shelf (a duel's, in practice): plaque (name over life) beside timeline (steps over counts and turn); the plaque never shrinks for tiles. `Shelf::box_size` measures from the ledge.
- Steps group into CR 500.1's five phases: `tile_gap` inside, `phase_gap` between (Split 3 vs 15). Main tiles `MAIN_SPAN` (2.0); width is one division in `tile_width_on`, never capped `flex_grow`; Split tiles grow together to a cap, then slack goes to phase gaps.
- Tree rebuilt per density change, box placed per frame; `stretch_step_tiles` pairs with `place_seat_bars`, or tiles freeze stale.
- Untap/cleanup tiles are dead: `grants_priority` false, `PhaseOrders::toggle` refuses; no fill, frame, stop marker; ink `DEAD_INK` (0.28); no `SeatStep`/`Feel`, so inert (CR 502.4, 514.3a). Both rows arranged in `settingsui.rs`; a bar shows its own turn's.

## Interaction

- Combat focus: `Interaction::toggle` pairs against it, `cycle_focus` moves it, tapping a planeswalker/attacker sets it. Wire `input.rs` and `hud.rs`; `toggle` and `confirm` share pairing state.
- `combat.rs` builds `Line`s from `view.combat` + unsent `assignments` (`standing`). `combatlines.rs` draws unlit quads (no `bevy_gizmos`) from live `Transform`s, never `Motion::target`. `Combat::tallies` (`tally_at`), the only arithmetic docs/design.md §6 allows, counts merely proposed blocks: "what reaches me if I block here".
- `manaplan.rs` matches lands (Kuhn) over `manasources.rs`; each step is an offered action re-checked against current `LegalActions` (`ManaRun`). No Phyrexian life; refuse `{X}`/`{S}`/restricted; a two-of-one-colour source counts once.
- Hand: playable (gold), reachable (indigo, taps first), activatable (`glow::ACTIVATABLE`, light on the felt round a permanent). The hand draws no glow: its halo says playable/reachable, and an armed card rises (`ARMED_RAISE`). Indigo passes `timing.rs` (`sorcery_lock`) and `targeting.rs` (CR 601.2c; withholds only on proof); `castmodes::parts_payable` refuses when unsure.
- A CR 605.3a payment window is `Pending::Priority` with mana abilities; `PlayerView::owed` holds the cost. Show `Phrase::PayOrPass`, owed pips beside the mana pool (same register, scale), `WILL_TAP` on `manaplan::plan`'s lands; pass the total. `Duel::proposing` is an enum; an armed deed beats a window, which arms nothing (§"A payment window…").
- `abilities.rs` lists only `LegalActions`, registry-labelled, never "Ability N". One option fires on the click; several open a prompt-bar chooser (by position, rebuilt from current `LegalActions` on press).
- Costs beyond own tap arm (`Duel::armed`), then send; Esc cancels; re-resolved against `LegalActions`; land drops and own-tap-only abilities are one click (`activate_card`, `arm_ability`; docs/keyboard-map.md §Arming).
- `ARMED` holds still, `ACTIVATABLE` travels, `WILL_TAP` marks lands; armed drops `ACTIVATABLE`. `CardGroup::activatable` needs every member.

## On the card

- Strip (`cardrail.rs`: `Strip` = marks, chip (swing), label; fifteen marks in `MARK_ORDER`, with hexproof, indestructible and shroud appended as 12–14. Marks are appended, never inserted, because the index is the GPU bit and the atlas cell. `ROW_MAX`; rows open upward) and plate (`cardplate.rs`). Client-core decides and `label_strip` draws; tests catch WGSL drift.
- Marks: Mana-font glyphs as a distance field (`markatlas.rs`), scale only; codepoints enter only via doors like `MARK_GLYPHS` (docs/legal.md §2a).
- The plate (P/T, or loyalty behind a gilt rim) is its own quad (`platemat.rs`, `plate.wgsl`/`plate_ui.wgsl`) keyed on `PlateWords` (one clamped `u32`: three 10-bit numbers, two kind bits; and its ink), shown only where the print can't say it (`Corner::shows_plate`: a changed body, damage, no print, covered in a fan, loyalty, lore). `cardplate::plate_rect` places it from `LanePacking::plate_room`: beside the printed box, on its own card in a fan (foot 0.880, above the artist and © lines), upright under a tapped card, none where neighbours fill the air (`layout::tests::plates`, `no_plate_lies_on_a_print_drawn_under_it`). An opponent's plate and badge read upright to the camera (`table::reads_upside_down` → `cardplate::PLATE_TURNED`, face turned in the shader, footprint unchanged; `every_plate_and_badge_reads_upright_to_the_eye`); strips stay as they lie. The preview shows it too; damage fills it to damage/toughness. Numerals: AlegreyaSans-Bold, atlas cell 12+, `lnum`+`tnum`, `TEXT_ADV` pinned by `the_advances_are_the_shipped_font_s_own`.
- Deathtouch greens power (`Tone`); toxic unreachable.
- The swing shows net counter P/T (green grown, violet shrunk; CR 704.5q); other counters nowhere (docs/observed-faults.md 58).
- A saga (lore counters, CR 714) takes the plate; `Corner::of`/`of_object`.
- Identity crests (`cardcrest.rs`: token, copy, commander) are squares of paper at the strip's end, after the marks. They pack (a lone commander takes the first). The paper is the reading (verdigris, violet, oxblood), and the glyph is ink on it. Linear colours; a sheen must beat ~20 noise levels.
- A summoning-sick creature (`board::asleep`, creatures only, CR 302.6) wears the moon on its strip, and its plate inks moon-grey (`PLATE_NIGHT`).
- Offers (`glow::OFFERS`: activatable, reachable, armed, will-tap) are light on the felt (`floormat.rs`, `floor.wgsl`): a child quad under the card at `FLOOR_RUNG`, never drawn on the card material. The count badge (`badgemat.rs`) stands at the top-right corner, outside the card: `Above` in a duel (upright on a tapped card, `table::keep_upright`, which also keeps the plate upright) and `Beside` at a ring, where a merged card holds the gap after it (`HELD_PITCH`). A merged group stands on a pile of slabs jogged by `PILE_JOG` 0.08 (`SLAB_EDGE_COLOR`).
- A protected permanent wears a shell (`shellmat.rs`, `shell.wgsl`). Rim, rings and wall are exactly transparent over their own print (a view-ray mask; `every_colour_but_the_domes_carries_the_mask`); a dome is glass over its whole card (the owner, 25.09, `docs/legal.md` §3). `table::fit_the_shells` stands a shell up or lays it on the felt from the live transforms, and one guard, `shellmat::shell_stands` over a profile that follows the throw's real direction, keeps every shell off other prints. Indestructible is a darksteel rim, a flat lip then a quarter-round bevel, that lies down as a rod. A standing dome whose foot is on the felt, and the wall, cast shadows on the felt (`ShellKind::Shade`, `SHADE_RUNG`): under every face, masked, a dome's faded out before a hover lifts it to a face (`a_shadow_lies_on_the_felt_outside_its_card`).
- Hexproof stands under a blue dome and shroud under a violet one (shroud swallows hexproof; `shellmat::Dome` rows, and #302 adds more). A dome stands at the tallest of `DOME_STEPS` that lands on no print and lies down last, as a band of plates in its colour (crisp, unlike the offer's soft light on the felt). If steel and dome both lie, they share the band inside out (0.10–0.13 and 0.13–0.16). Use Blend, never Add: Bevy's Add is premultiplied, so alpha 0 still adds colour.
- Defender stands behind a brick wall on the felt past its top edge (`shellmat::wall_mesh`), at most `WALL_HEIGHT` 0.08 high: lower than every face (`RIM_DROP`, 0.083), so any print in front hides it by depth. It is solid (writes depth, `WALL_RUNG` first), and `table::wall_pose` never lifts or grows it with its card (`a_wall_never_draws_over_a_print`).
- New card marks join `ObjectSummaryKey`.

## Stack and HUD

- Stack entries (`hud/stack.rs`): spell or source, then targets; `BoardModel::from_view` resolves `StackTarget` via `PlayerView::object`; art joins `required_images`; player targets: `name: None`.
- `StackText{face,line,of}` indexes the card in `rules` (`RulesFace`; for a copy the copied card, CR 707.2), not the source's. `cardtext::sentence` is the one door (ability sheet, stack, cast chooser): the player's printing where `baylee_cardtext::align` pairs it with the compiled English Oracle, else that Oracle's line, never blank. A host line whose `of` is not this build's count is refused; the stack then places the ability by its `AbilityRef` in this build's line table (the sheet and chooser always do), and keeps the source's name only without one. Use `StackText.face`, not the current face (CR 113.7a; §"Which ability is on the stack").
- `hud::ledge` rebuilds the Actions Bar per sentence (`LedgeRevision`); `drawer` (centre), `players` strip (left, #264: a `PlayerTab` button per seat, three eased edges: turn top, wait border, camera bottom), `pool` strip (right), `tray` (log and zones doors in the bar left of the burger) and `menu` (burger: draw offer, concede, version) keep their own revisions and survive it; players and pool share `strip_node`. Sort buttons at `EDGE`, `TOOL_H`; `arrange` takes `tools_reserved(window_w)`.
- `hud::tray` is the zone dialog, `ledge::tray` its button; the `hud::zones` rename (409 occurrences) is unrequested; name map in `ledge::tray`'s module doc. `Browser::close` minimises; G, Escape, head button, tray ask `may_be_put_away`.
- `sync_tray` sets `TrayReveal::closing`; `reveal_tray` (`.after(sync_tray)`) flies the sheet home and despawns. Gate `drawn && !closing`; tear down `TrayVeil`, not every `TableVeil`.
- Head: `TrayMinimise`, `TrayMaximise`; the corner only drags. Maximise lerps `Placement` (`glide_the_sheet`), never `UiTransform`; new head controls join `tray_drag`'s exclusions.
- Padded parchment: `hud::sheet_surface()`, not `sheet()`. `Text` in controls: `Pickable::IGNORE`. Answer buttons: `flex_grow: 1`, `flex_basis: 0`.

## Text, input, language

- Alegreya Sans for interface, Faustina (own italic file) for card text (overrides docs/design.md §1.2). Pass nominal sizes (`UI_SCALE` 1.2, `SERIF_SCALE` 1.1); re-measure font-derived numbers (§"Two families").
- `prose::bracketed` greys closed brackets only.
- Handlers ask actions: `Keymap` (`baylee-client-core/src/prefs.rs`) resolves only in `keys.rs` (raw `KeyCode` bypasses text-field guards); deserialising drops only unknown actions.
- Keymap, standing orders, automation: `GET/PUT /settings`, edited in `settingsui.rs`. `ClientSettings.lang` feeds catalog `lang=` and `Lang::of`.
- UI strings are `Phrase` arms (`i18n.rs`): missing German fails to compile; tests check coverage and `{0}`/`{1}`.
- Lobby via `Lobby::note`, shell via `tell`/`unseat_because`; gateway errors stay untranslated. Refusals: `Refusal::Said(Phrase)` (ours) or `Verbatim(String)` (theirs; legitimate, not a count to drive to zero), never bare `String`. Identifiers (`"sharp"`) keep wire spelling (§"The interface's own words").
