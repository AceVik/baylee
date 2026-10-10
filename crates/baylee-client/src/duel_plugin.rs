//! The duel's plugin: its configuration, its system sets and the schedules it installs.

#[allow(clippy::wildcard_imports)] // the parent's own vocabulary
use super::*;

/// How the duel is configured when it opens.
#[derive(Resource, Clone, Debug)]
pub struct DuelConfig {
    /// Texture budget in bytes.
    pub texture_budget: usize,
    /// Whether to draw the debug overlay.
    pub debug_overlay: bool,
}

impl Default for DuelConfig {
    fn default() -> Self {
        Self {
            texture_budget: textures::default_budget_bytes(),
            debug_overlay: false,
        }
    }
}

/// System sets, so an embedding application can order its own work around the
/// duel's without depending on individual system names.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DuelSet {
    /// Draining the host and rebuilding the board model.
    Sync,
    /// Turning input into actions.
    Input,
    /// Updating the scene and the overlay.
    Present,
}

/// The duel client, as a plugin.
#[derive(Default)]
pub struct DuelPlugin {
    /// Configuration applied on insert.
    pub config: DuelConfig,
}

/// Everything the duel *draws*: the scene, the sky, the overlay, and the
/// three animations that live above the retained tree.
///
/// A function of its own rather than another link in `build`'s chain,
/// because it is the longest of the four sets and it grows every time the
/// client learns to animate something. The inner tuple is the overlay's own
/// animations — a system tuple holds twenty and the outer list has already
/// outgrown one. `ease_the_stack_in` runs after the rebuild deliberately: a
/// stack row spawned this frame is spawned at rest, so without the ordering
/// it is drawn once at full strength before its arrival is ever applied.
#[allow(clippy::too_many_lines)] // one registration list, which grows with every animation
pub(super) fn add_present_systems(app: &mut App) {
    app.init_resource::<hud::StackFold>()
        .init_resource::<hud::TrayReveal>()
        .init_resource::<hud::MenuRevision>()
        .init_resource::<hud::LogRevision>()
        .init_resource::<hud::revealed::RevealRevision>()
        .init_resource::<input::TrayGlide>();
    app.add_systems(
        Update,
        (
            (
                table::track_canvas,
                table::track_proposals.before(table::sync_scene),
                rowbar::follow_the_rows.before(table::sync_scene),
                rowbar::sync_row_bars.after(table::sync_scene),
            ),
            // Ahead of both things that draw a card, so a card arriving is
            // placed with its sheen already decided rather than a frame late.
            sheen::watch_for_arrivals
                .before(table::sync_scene)
                .before(hud::sync_overlay),
            table::sync_scene,
            (table::sync_zones, table::sync_library_fan).chain(),
            (
                table::sync_table,
                dial::turn_the_dial,
                feltmat::install_the_warp,
            )
                .chain(),
            sky::hang_sky,
            sky::sync_sky,
            sky::light_the_table.after(sky::sync_sky),
            // One entry and not four: a system tuple holds twenty and this
            // list is at its limit. Chained rather than merely ordered
            // because that is what the first pair is — a card that left this
            // frame is moved once before it is counted against its own clock,
            // so a table running at ten frames a second still shows the exit
            // instead of skipping it. The last reads where the glide left
            // each card: a flying one's shadow.
            (
                combatfx::animate.before(table::sync_scene),
                combatfx::age.before(table::sync_scene),
                table::glide.after(table::sync_scene),
                table::retire,
                table::ground_the_shadows,
                table::keep_upright,
                // Where the camera is this frame, as well as the cards.
                table::fit_the_shells.after(table::apply_camera_rig),
            )
                .chain(),
            // After the glide, and deliberately: a line is welded to where
            // its two cards *are* this frame, so it has to be computed once
            // they have moved.
            combatlines::sync_combat_lines.after(table::glide),
            combatlines::sync_focus_ring.after(table::glide),
            (table::frame_table, table::apply_camera_rig).chain(),
            // One entry and not two, because the tuple is at its twenty: a card
            // that has left the hand is taken out of the row before the row
            // is rebuilt, and chaining is what says so. The commands of the
            // first are queued before the commands of the second, so the
            // node is out of the hand zone when the hand zone is despawned.
            (depart::send_off, hud::sync_overlay).chain(),
            hud::apply_hand_scroll,
            (
                hud::light_the_current_step,
                hud::flash_the_designation,
                hud::ease_the_stack_in.after(hud::sync_overlay),
                hud::fold_the_stack.after(hud::sync_overlay),
                hud::reveal_tray.after(hud::sync_tray),
                // The same ordering, and the same reason: the slip under
                // the hover preview is spawned written, and this is what
                // takes the ink back off and washes it on.
                // And after the rebuild too: a preview rebuilt this frame is
                // stood back where its text had been scrolled to, and its
                // scrollbar shown if the text runs over (#259).
                // And a stack entry's sentence the same way: stood where it
                // had been scrolled to, its keys' steps taken, and its bar
                // shown while it runs over.
                (
                    hud::wash_the_slip_in,
                    hud::keep_the_preview_scrolled,
                    face::show_scrollbars,
                    hud::stack_text,
                )
                    .after(hud::sync_overlay),
                // After the rebuild for the reason `ease_the_stack_in` is:
                // a hand card spawned this frame is spawned where the card
                // already was, and this is what moves it from there.
                touch::settle.after(hud::sync_overlay),
                // After the rebuild too, and for the mirror of that reason:
                // a card taken out of the row this frame is written to its
                // window position by `send_off` and moved from there by
                // this, and a flight advanced before the hand had let go of
                // it would spend its first frame twice.
                depart::fly.after(hud::sync_overlay),
                // The seat bars are ink pinned to a rectangle of felt, so
                // they are measured from the rig the camera was just set
                // from and placed in the same schedule. `bevy_ui` runs its
                // layout *before* transform propagation, so a placer reading
                // the camera's propagated `GlobalTransform` in `PostUpdate`
                // would write a position the layout had already read past,
                // and the ink would swim a frame behind the felt.
                (
                    hud::measure_shelves,
                    hud::sync_seat_bars,
                    hud::place_seat_bars,
                    hud::stretch_step_tiles,
                    // Each seat's clock beside its plate: built when the
                    // seats change, then counted and stood beside the plate
                    // `place_seat_bars` has just placed, from the same rig.
                    hud::sync_plate_clocks,
                    hud::tick_plate_clocks,
                    hud::chosen_type::sync.after(table::glide),
                    hud::suspended::sync,
                    hud::describe_phase,
                    hud::highlight_player,
                )
                    .chain()
                    .after(table::apply_camera_rig),
                // The ability sheet is pinned to a *card* rather than to a
                // rectangle of felt, and to where that card **stands** rather
                // than the pose it is drawn in: `table::CardRest` is written
                // by `sync_scene` before a hover or an arming lifts the card,
                // so the paper is not dragged about by the hand crossing the
                // thing it is describing. After that write, and after the rig
                // for the reason the bars are.
                //
                // `zoom_the_sheet` is between the two on purpose. It is what
                // despawns a closing sheet, so it has to run after the system
                // that hands one over; and the scale it writes is one the
                // placer never reads — that one is about *where* the paper
                // is, this one about how much of it has arrived.
                (
                    hud::sync_granted_sheet,
                    hud::sync_ability_sheet,
                    hud::zoom_the_sheet,
                    hud::place_ability_sheet,
                    hud::focus_granted_sheet,
                )
                    .chain()
                    .after(table::apply_camera_rig)
                    .after(table::sync_scene),
                // A life total changing is drawn over the cell that carries
                // it, so this runs once the bar holding that cell has been
                // rebuilt and placed. It reads `bevy_ui`'s own layout for
                // where the cell is, which is a frame old for the reason
                // above — and a frame is nothing to a number that hangs for
                // a second, where guessing the position from the shelf and
                // the tilt would be the bar's layout written out twice.
                lifeflash::flash_life_changes.after(hud::place_seat_bars),
                // The veil behind a dialog that holds the whole answer. After
                // the rebuild, because the veil *is* part of the retained tree
                // and is spawned clear: how far the fade has risen lives in
                // `hud::Veil`, where a rebuild cannot reach it, and this is
                // what paints it on. See `tray::spawn_veil`.
                // The shelf is filled after the tree that holds it is built, and
                // has a revision of its own for the reason `LedgeRevision`
                // gives: `HudRevision` counts the hover, and a question
                // rebuilt on every pointer move would lose the warmth under
                // the pointer that is about to press it.
                hud::sync_ledge.after(hud::sync_overlay),
                // And the drawer after the shelf, because the centre it stands
                // over is what the shelf has just worked out.
                hud::sync_drawer.after(hud::sync_ledge),
                // And the movement after the reading, on the same frame: a
                // panel `sync_drawer` has just sent away has to be able to
                // leave, and a panel it has just spawned is drawn small on
                // the frame it first appears rather than a frame later.
                hud::zoom_the_drawer.after(hud::sync_drawer),
                // The owed strip, on its own revision, after the shelf it
                // hangs beside — and its movement after that, for the
                // drawer's reason: the strip has to be able to fold away.
                // Nested, and it has to stay nested: `add_systems` takes a
                // tuple and a tuple of systems is implemented up to twenty.
                // This set was at twenty, so the pair goes in together rather
                // than the next system to be added failing to compile for a
                // reason that has nothing to do with it.
                (
                    hud::sync_pool.after(hud::sync_ledge),
                    // After the shelf, because the cell it writes into is one
                    // `sync_ledge` spawns: ordered the other way, a countdown
                    // appearing would show an empty cell for its first frame.
                    hud::count_down_the_decision.after(hud::sync_ledge),
                ),
                // The two of them nested as one element on purpose: bevy
                // implements `IntoScheduleConfigs` for tuples up to twenty,
                // and this tuple was at nineteen. A nested tuple is a tuple
                // of systems like any other, so the grouping costs nothing at
                // run time and the pair that belongs together is the one that
                // pays for the ceiling.
                (
                    hud::grow_the_pool.after(hud::sync_pool),
                    // The game menu's panel and its movement, in the pool's
                    // nest for the pool's reason — the tuple above is at
                    // bevy's twenty — and ordered after the shelf because the
                    // burger that opens it is drawn there. It hangs off
                    // `HudRoot` rather than off the shelf, so it needs
                    // nothing the shelf worked out; what it must not do is
                    // stand over a shelf that is not there yet.
                    hud::sync_menu.after(hud::sync_ledge),
                    hud::grow_the_menu.after(hud::sync_menu),
                    // The game log's panel, in the same nest for the same
                    // reason, and after the shelf because it stands on the
                    // strip that hangs off it. `follow_the_log` reads where
                    // the rows the sync just added have been laid out, which
                    // is the frame after they were spawned.
                    hud::sync_log.after(hud::sync_ledge),
                    hud::grow_the_log.after(hud::sync_log),
                    hud::follow_the_log.after(hud::sync_log),
                    hud::hover_log_links.after(hud::sync_log),
                    hud::update_ai_log,
                    // Cards another seat revealed, after the shelf for the
                    // log's reason (both hang off the overlay's root).
                    hud::revealed::sync.after(hud::sync_ledge),
                ),
                // The tray's doors stand in the shelf's row but not in its
                // layout, so they need nothing the shelf worked out — but
                // they are ordered after it anyway, because they are spawned
                // by the same rebuild the shelf is and a frame where they
                // exist and the shelf does not would draw buttons standing
                // on air. The players' strip (#264) hangs off the same shelf
                // for the same reason, and its three edges move after it is
                // filled. Nested with the doors, because the tuple around
                // them is at bevy's twenty.
                (
                    hud::sync_tray_strip.after(hud::sync_ledge),
                    hud::sync_players.after(hud::sync_ledge),
                    hud::glow_the_players.after(hud::sync_players),
                    hud::show_the_tags.after(hud::sync_players),
                    hud::show_priority_switch,
                    // A chip's or a plate's name in words, under a pointer
                    // resting on it.
                    hud::show_hint.after(hud::sync_players),
                ),
                // The zone dialog, on a revision of its own for the same
                // reason as the shelf and with a louder symptom: the dialog
                // is a hundred rows, and a tree rebuilt on every pointer move
                // despawned the row under the pointer *as the pointer reached
                // it* — the replacement starting at `warmth: 0` and waiting a
                // frame for picking to say `Over` again. The owner reported
                // it as the dialog being unstable.
                //
                // After the overlay, because it hangs its two nodes off that
                // system's root; `sync_overlay` passes them over by marker
                // the way it passes over the shelf and the drawer.
                hud::sync_tray.after(hud::sync_overlay),
                hud::dim_the_table.after(hud::sync_overlay),
                // The end screen settles as that veil rises, off the very
                // number `dim_the_table` has just written: one movement, one
                // rate, one `reduce_motion`.
                hud::settle_the_sheet.after(hud::dim_the_table),
            ),
            textures::drive_preloads,
            textures::load_the_card_back,
            textures::note_load_states,
            textures::retry_failed_loads,
        )
            .in_set(DuelSet::Present)
            .run_if(not(in_state(DuelPhase::Closed))),
    );
    // Its own call because the tuple above is at its twenty, and its own
    // *system* because it belongs to none of them: it is where a frame stops
    // deciding what is worth hearing and hands it over. In `Present` rather
    // than `Sync` for the one reason that matters — that is what lets a cue
    // be taken back, since every source of one and everything that answers a
    // question have run by the time this does.
    app.add_systems(
        Update,
        sound::play_the_cues
            .after(combatfx::age)
            .in_set(DuelSet::Present)
            .run_if(not(in_state(DuelPhase::Closed))),
    );
    // The cues' buffers, synthesised off the main thread since startup,
    // arrive whenever the last is done: the lobby is open by then.
    app.add_systems(
        Update,
        sound::collect_the_voices.run_if(resource_exists::<sound::Voicing>),
    );
}

/// Everything a hand does, in the order the frame has to read it in.
///
/// Lifted out of [`DuelPlugin::build`] for [`add_present_systems`]'s
/// reason and no other: this one tuple carries five ordering constraints
/// and about as many paragraphs saying why, and a `build` that holds it
/// inline is a function nobody can read the shape of. Registration order
/// says nothing in Bevy — every order that matters here is written as a
/// `before` or an `after` — so where the call stands does not either.
pub(super) fn add_input_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            // Before the key path, and for the reason the lobby's
            // sits there too: on a platform that owns the typing the
            // client must not also read raw keys, or a character is
            // entered twice.
            input::browser_softkeys,
            // Before the key path, so the frame the sheet opens on is
            // already one the filter box owns. A letter that reached
            // `look_around` instead is a display toggle or an engine
            // answer fired out of somebody's search term.
            input::browser_takes_the_keyboard.before(input::keyboard),
            input::keyboard,
            // Before the click, and it has to be: a press and the
            // click it turns into arrive on the same frame, so a
            // finger put down *after* its own tap had been answered
            // would leave the card pressed with nothing to lift it.
            touch::watch_the_finger.before(input::pointer),
            // Before `pointer`, and on the *press* rather than the
            // click it becomes: a click on another card has to close
            // this sheet and then open that one, which is two things
            // in that order and not one thing twice.
            input::close_the_sheet_on_a_press_outside_it.before(input::pointer),
            // And the game menu, on the same mechanics and the same
            // ordering: a press outside it shuts it, and a press on its
            // own button is spared so the click can toggle.
            input::close_the_menu_on_a_press_outside_it.before(input::pointer),
            input::pointer,
            input::pointer_hover,
            // `input::camera_controls` used to stand here, and its
            // absence is the point: the owner asked for the general
            // camera movement to go on 14.09.2026, keyboard included,
            // so nothing a hand does reaches the rig any more.
            // `hud::scrolls` no longer shares the wheel with anything
            // and there is no order left to get wrong.
            // Before the wheel, so a wheel over a card the pointer has just
            // reached scrolls that card's preview from its top (#259).
            hud::follow_the_hover
                .after(input::pointer_hover)
                .before(hud::scrolls),
            input::preview_resize,
            input::tray_drag,
            // After the drag, for the reason `glide_the_sheet`'s own
            // doc gives: both write the sheet's `Node` outside the
            // revision, and a frame in which the two disagreed would
            // be a frame the later one wins by accident rather than
            // by a stated order. They cannot both be running — the
            // maximise button is excluded from `tray_drag`'s press —
            // so the order costs nothing and says so.
            input::glide_the_sheet.after(input::tray_drag),
            face::track_modifier,
        )
            .in_set(DuelSet::Input)
            .run_if(in_state(DuelPhase::Playing)),
    );
    // The wheel, on its own because it is the one input that outlives the
    // game: the end sheet's log scrolls (#262), and every other system above
    // would be answering a question nobody is asking any more.
    app.add_systems(
        Update,
        hud::scrolls
            .in_set(DuelSet::Input)
            .run_if(in_state(DuelPhase::Playing).or_else(in_state(DuelPhase::Finished))),
    );
}

impl Plugin for DuelPlugin {
    #[allow(clippy::too_many_lines)] // the client plugins are registered together
    fn build(&self, app: &mut App) {
        add_present_systems(app);
        arrangement::plugin(app);
        add_input_systems(app);
        // Shared with the lobby, which is a separate plugin and may already
        // have installed it.
        prefs::install(app);
        music::install(app);
        ambience::install(app);
        loading::install(app);
        flip::install(app);
        // The game log's scrollbar is Bevy's own, so its thumb can be dragged
        // (#262). The lobby installs the same plugin, and either may be first.
        if !app.is_plugin_added::<bevy::ui_widgets::ScrollbarPlugin>() {
            app.add_plugins(bevy::ui_widgets::ScrollbarPlugin);
        }
        app.add_plugins(cardmat::CardMaterialPlugin)
            .add_plugins(markatlas::MarkAtlasPlugin)
            .add_plugins(marksmat::MarksMaterialPlugin)
            .add_plugins(badgemat::BadgeMaterialPlugin)
            .add_plugins(platemat::PlateMaterialPlugin)
            .add_plugins(floormat::FloorMaterialPlugin)
            .add_plugins(shellmat::ShellMaterialPlugin)
            .add_plugins(shellui::ShellUiPlugin)
            .add_plugins(feltmat::FeltMaterialPlugin)
            .add_plugins(frontal::FrontalPlugin)
            .add_plugins(matmat::MatMaterialPlugin)
            .add_plugins(arrowmat::ArrowMaterialPlugin)
            .add_plugins(sky::SkyPlugin)
            // Without this nothing on the 3D table can be pointed at, ever.
            //
            // Bevy's UI picking backend is on by default and its *mesh* one is
            // not, so the hand zone — which is UI nodes — answered the pointer
            // while the battlefield, the stack of a hovered permanent and
            // every pile beside a mat did not: `Pointer<Over>` and
            // `Pointer<Click>` simply never fired for a `Mesh3d`. That is why
            // `Interaction::activate` could be written, wired to `input.rs`,
            // and still leave a Forest inert under the cursor, and why the
            // preview only ever appeared for cards in hand. Measured rather
            // than guessed: hovering a hand card reports its object, hovering
            // an opponent's land at the pixel the card is drawn on reports
            // nothing at all.
            //
            // `require_markers` stays `false` — the default, and the one the
            // `Pickable::IGNORE` already on the contact shadows was written
            // against. Everything on the table that is not a card carries
            // that marker, so the felt itself never answers a click, and a
            // card needs no marker of its own. Measured that way round too:
            // with a `Pickable::default()` added to every card the hover was
            // no different, so it is not there.
            //
            // The `mesh_picking` cargo feature this needs cannot be dropped
            // by a later `default-features = false` audit without the build
            // saying so — the path below names the module the feature gates.
            .add_plugins(bevy::picking::mesh_picking::MeshPickingPlugin)
            .init_state::<DuelPhase>()
            .insert_resource(self.config.clone())
            .insert_resource(settings::ClientSettings::load())
            .init_resource::<Duel>()
            .init_resource::<hud::PreviewScroll>()
            .init_resource::<hud::StackTextScroll>()
            // Both are written by systems that run every frame; a missing
            // resource here is a panic at the table, not a compile error.
            .init_resource::<table::SceneIndex>()
            .init_resource::<table::ZoneWatch>()
            .init_resource::<table::CameraRig>()
            .init_resource::<table::ShownRig>()
            .init_resource::<table::CameraPose>()
            .init_resource::<dial::DialReport>()
            .init_resource::<Reconnect>()
            .init_resource::<sheen::Sheen>()
            .init_resource::<touch::Touched>()
            .init_resource::<hud::HudRevision>()
            .init_resource::<hud::StackMotion>()
            .init_resource::<hud::SlipWash>()
            .init_resource::<hud::DesignationFlash>()
            .init_resource::<hud::Shelves>()
            .init_resource::<hud::BarRevision>()
            .init_resource::<hud::LedgeRevision>()
            .init_resource::<hud::LedgeLayout>()
            .init_resource::<hud::DrawerRevision>()
            .init_resource::<hud::PoolRevision>()
            .init_resource::<hud::PlayersRevision>()
            .init_resource::<hud::StripRevision>()
            .init_resource::<hud::TrayRevision>()
            .init_resource::<hud::SheetRevision>()
            .init_resource::<hud::Veil>()
            .init_resource::<textures::Preload>()
            .init_resource::<cardtext::CardTexts>()
            .init_resource::<cardtext::TextGateway>()
            .insert_resource(face::FaceMode::initial())
            .init_resource::<face::FaceBuilds>()
            .init_resource::<combatlines::LineAssets>()
            .init_resource::<combatlines::FocusAssets>()
            // Shared with the lobby, which may already have installed it: the
            // table has one text field of its own, the browser's filter box.
            .init_resource::<softkeys::SoftKeyboard>()
            .add_message::<DuelCommand>()
            .add_message::<DuelReport>()
            .init_resource::<crate::manaui::ManaCoverage>()
            // The one door between a symbol's glyph and its fallback letters,
            // before UI and text layout so nothing flashes the other form.
            .add_systems(
                PostUpdate,
                crate::manaui::ink_the_marks.before(bevy::ui::UiSystems::Prepare),
            )
            .configure_sets(
                Update,
                (DuelSet::Sync, DuelSet::Input, DuelSet::Present).chain(),
            )
            .add_systems(
                Startup,
                (
                    textures::setup,
                    hud::setup_fonts,
                    hud::setup_sheets,
                    // Once, on the frame the app opens: thirty-seven
                    // buffers of arithmetic, started on the compute pool
                    // and collected by `sound::collect_the_voices`. See
                    // `sound`'s header for why they are computed and not
                    // shipped, and what the count buys.
                    sound::voice_the_cues,
                ),
            )
            .add_systems(
                Update,
                (
                    // The cue queue's clock, before anything can decide a cue.
                    sound::tell_the_cues_the_time.before(poll_host),
                    handle_commands,
                    poll_host,
                    keep_the_table_connected.run_if(duel_is_live),
                    run_mana_plan,
                    // After the run and before the HUD is built, which is
                    // both halves of where it belongs: the run is what puts
                    // the mana up and sends the cast, and a frame drawn
                    // between the engine's question and this answer would
                    // flash the engine's own chooser over a choice the player
                    // already made.
                    answer_the_chosen_cast_mode,
                    run_autopilot,
                    flush_outbox,
                    cardtext::request,
                    cardtext::poll,
                    cardtext::poll_scryfall,
                )
                    .chain()
                    .in_set(DuelSet::Sync),
            )
            .add_systems(
                OnEnter(DuelPhase::Opening),
                (table::forget_the_last_table, table::spawn_stage),
            )
            // The end screen. Built on the edge because a result never
            // changes, and taken down on the way out of `Finished` — which
            // covers the way to `Closed` too, so it needs no line in the
            // teardown below.
            .add_systems(OnEnter(DuelPhase::Finished), hud::spawn_finish)
            .add_systems(OnExit(DuelPhase::Finished), hud::despawn_finish);
        tear_the_table_down(app);
    }
}

/// What closing a duel takes down: the stage, the overlay, the game's
/// printings.
///
/// Its own function so that `teardown_tests` runs the very schedule a real
/// client runs on leaving a table, rather than a copy of it that could drift.
pub(crate) fn tear_the_table_down(app: &mut App) {
    app.add_systems(
        OnEnter(DuelPhase::Closed),
        (
            table::despawn_stage,
            hud::despawn_overlay,
            textures::reset_game,
        ),
    );
}
