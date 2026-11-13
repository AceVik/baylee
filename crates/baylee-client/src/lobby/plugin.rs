//! Lobby system registration.
#[allow(clippy::wildcard_imports)]
use super::*;

impl Plugin for LobbyPlugin {
    #[allow(clippy::too_many_lines)] // one plugin: every system the lobby runs, in order
    fn build(&self, app: &mut App) {
        // The keymap is the account's, and the account is signed into here —
        // shared with the duel, whichever of the two got there first.
        if !app.is_plugin_added::<bevy::ui_widgets::ScrollbarPlugin>() {
            app.add_plugins(bevy::ui_widgets::ScrollbarPlugin);
        }
        crate::prefs::install(app);
        app.add_message::<bevy::window::FileDragAndDrop>();
        crate::ambience::install(app);
        // A deletion waiting for its Undo goes out as the client closes.
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Last, decks::flush_on_exit);
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, central::sync_gateways);
        app.add_systems(Update, lend_the_pool.after(poll));
        // Closing the application only disconnects. Explicit table actions
        // alone may depart; dropping the World must leave the seat resumable.
        crate::vista::install(app);
        app.add_plugins((memorial::CloudPlugin, memorial::BookPlugin));
        // The same orchestra continues through the lobby and every table.
        crate::music::install(app);
        // The report form stands over the lobby and every table (#309).
        crate::report::install(app);
        // Friends and chat, over the lobby and every table (beta.8).
        crate::social::install(app);
        crate::loading::install(app);
        crate::shellkit::install(app);
        crate::tour::install(app);
        // A shell chapter is set aside while a table stands.
        app.add_systems(
            OnExit(DuelPhase::Closed),
            |mut desk: ResMut<crate::tour::TourDesk>| {
                if desk
                    .run
                    .as_ref()
                    .is_some_and(|r| r.tour != baylee_client_core::tour::Tour::Table)
                {
                    desk.parked = true;
                }
            },
        );
        header::install(app);
        install_builder(app);
        // A `baylee:` link the lobby follows once it can (`links`): after
        // the mailbox, before the tree, so a question it raises is drawn the
        // same frame. The resource is the links plugin's; a lobby without
        // it (a test) simply never hears one.
        app.init_resource::<crate::links::Arrived>();
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(
            Update,
            links::follow
                .after(poll)
                .before(ui)
                .run_if(in_state(DuelPhase::Closed)),
        );
        // Restarting into an update, and coming back to where the player was.
        #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
        resume::install(app);
        crate::flip::install(app);
        app.init_resource::<thumbnails::Cache>()
            .init_resource::<thumbnails::ArtCache>()
            .init_resource::<dock::Surfaces>()
            .init_resource::<Mailbox>()
            .init_resource::<feed::Feed>()
            .init_resource::<ChatJoins>()
            .init_resource::<SoftKeyboard>()
            .init_resource::<Scrolled>()
            .init_resource::<focusing::Kept>()
            .init_resource::<UiRebuilds>()
            .init_resource::<shell::ScreenTransition>()
            .insert_resource(LobbyState::new())
            .init_resource::<hint::Hinted>()
            .init_resource::<front::FrontMotion>()
            .init_resource::<entrance::Entrance>()
            .init_resource::<front::FrontCast>()
            .init_resource::<front::StageAtRest>()
            .init_resource::<front::RowPlaces>()
            .init_resource::<front::terms::SheetFocus>()
            .init_resource::<front::keys::MovedByTheLobby>()
            .init_resource::<menus::MenuMemory>()
            .configure_sets(Update, crate::tour::TourSystems::Draw.after(ui))
            .add_systems(Startup, (ask_about_registration, ask_about_saved_gateways))
            // One chain, each link on its own condition: most of it is the
            // lobby's alone, and the links the settings screen needs run
            // while it stands over a table too (`table_settings`).
            .add_systems(
                Update,
                (
                    (feed::feed, chat_joins).run_if(in_state(DuelPhase::Closed)),
                    (poll, entrance::advance)
                        .chain()
                        .run_if(in_state(DuelPhase::Closed)),
                    localization::update.run_if(in_state(DuelPhase::Closed)),
                    (
                        watch,
                        play::discovery::animate_loading,
                        play::discovery::hover_cards.after(crate::ambience::feel),
                    )
                        .run_if(in_state(DuelPhase::Closed)),
                    softkeys.run_if(table_settings::lobby_or_over_table),
                    scrollbars::remember.run_if(table_settings::lobby_or_over_table),
                    (
                        crate::seatpanel::poll.run_if(in_state(DuelPhase::Closed)),
                        // Before the kit's walker reads the same keys: an
                        // Enter that activates a focused control (which may
                        // put the caret in a field) is not also the field's.
                        keyboard
                            .before(crate::shellkit::focus::FocusSystems)
                            .run_if(table_settings::lobby_or_over_table),
                        crate::tableseats::reconcile.run_if(in_state(DuelPhase::Closed)),
                    )
                        .chain(),
                    // A press asks the clipboard or the disk; the ask is
                    // answered in the same frame.
                    (
                        clicks.run_if(table_settings::lobby_or_over_table),
                        crate::buildui::transfer::act.run_if(in_state(DuelPhase::Closed)),
                    )
                        .chain(),
                    (
                        scrolls.run_if(table_settings::lobby_or_over_table),
                        scroll_the_preview.run_if(in_state(DuelPhase::Closed)),
                    ),
                    (hovers, hint::hint_hovers).run_if(in_state(DuelPhase::Closed)),
                    // The motion decides which panels are drawn, and the
                    // pose lands on the panels the rebuild just stood up.
                    (
                        source::keep_the_code,
                        front::move_front,
                        front::show_scene.before(crate::vista::paint),
                        ui,
                        shell::animate_screen_transition,
                        retrace_runs,
                        front::keys::lobby_to_kit,
                        front::pose_front,
                        front::glide_the_stage,
                        // After the rebuild, so a tile just stood up glows
                        // as the round says on its first frame.
                        front::door::beckon,
                    )
                        .chain()
                        .run_if(in_state(DuelPhase::Closed)),
                    // The settings over a table, where the lobby's tree
                    // stands no more: drawn where `ui` draws the lobby's.
                    table_settings::draw,
                    // After the rebuild, so a music switch it just stood up
                    // says the right thing on its first frame.
                    blink.run_if(table_settings::lobby_or_over_table),
                    dock::materialize.run_if(in_state(DuelPhase::Closed)),
                    (button_style::materialize, crate::shellkit::sheen::dress)
                        .run_if(table_settings::lobby_or_over_table),
                    // The duel's own runs only while there is a duel, and a
                    // text face in the deckbuilder shows its scrollbar too.
                    (
                        (
                            preview,
                            preview::animate_preview,
                            hint::hint_panel,
                            crate::face::show_scrollbars,
                        ),
                        (
                            gateway::sync_virtual_rows,
                            crate::buildui::virtual_rows::update,
                            crate::buildui::virtual_rows::in_deck,
                            crate::buildui::focus::light_the_cursor,
                            crate::buildui::focus::scroll_to_the_cursor,
                            crate::buildui::focus::place_menus,
                        ),
                        (
                            thumbnails::load,
                            thumbnails::prefetch_art,
                            thumbnails::load_art,
                            thumbnails::fade_art,
                            deck_frame::animate_deck_frames,
                        ),
                        thumbnails::quantities,
                        waiting,
                    )
                        .chain()
                        .run_if(in_state(DuelPhase::Closed)),
                )
                    .chain(),
            )
            .add_systems(
                Update,
                menus::animate.after(ui).run_if(in_state(DuelPhase::Closed)),
            )
            // After `Update` and layout, so that newly built panels have their
            // computed width and whatever else wrote a colour this frame has
            // written it first.
            .add_systems(
                PostUpdate,
                (
                    front::pose_front,
                    front::fade_front,
                    front::fade_primary_surfaces,
                    front::terms::read_the_terms,
                )
                    .chain()
                    .after(bevy::ui::UiSystems::Layout)
                    .run_if(in_state(DuelPhase::Closed)),
            )
            // After layout, which is what says whether a page runs over.
            .add_systems(
                PostUpdate,
                scrollbars::show_when_needed
                    .after(bevy::ui::UiSystems::Layout)
                    .run_if(table_settings::lobby_or_over_table),
            )
            .add_systems(
                Update,
                (leave_clicks, leave_keys).run_if(in_state(DuelPhase::Finished)),
            )
            // The front door's half of the kit's focus (`front::keys`), and
            // the terms sheet's: after the walker moved focus, before the
            // tree is drawn again.
            .add_systems(
                Update,
                (
                    // After `Esc` or a click put the settings over a table
                    // away, so the search lets the caret go the same frame;
                    // and in every phase, so a close this group's own Enter
                    // made is followed the next frame even at a table, where
                    // the lobby's systems stop with the screen. It writes
                    // only when the two flags differ.
                    crate::settingsui::keys::settings_over
                        .after(table_settings::close_on_escape)
                        .after(clicks),
                    // Enter on a settings control, and the kit's focus and
                    // the lobby's caret as one (the search): over a table too.
                    (
                        front::keys::activate_by_key,
                        front::keys::kit_to_lobby,
                        front::keys::unpark_on_placement,
                        front::keys::lobby_to_kit,
                    )
                        .chain()
                        .run_if(table_settings::lobby_or_over_table),
                    (
                        front::terms::follow_the_session,
                        front::terms::follow_the_language,
                        front::terms::terms_keys,
                        front::terms::read_the_terms,
                        front::terms::place_sheet_focus,
                    )
                        .chain()
                        .run_if(in_state(DuelPhase::Closed)),
                )
                    .chain()
                    .after(crate::shellkit::focus::FocusSystems)
                    .after(poll)
                    .after(scrollbars::remember)
                    .before(ui),
            )
            // The settings over a table (`table_settings`): asked for by the
            // game menu; `Esc` puts them away before the lobby's keyboard
            // reads it, and the table being put down puts them away too.
            .add_systems(
                Update,
                (
                    table_settings::open_asked.before(table_settings::draw),
                    table_settings::close_on_escape.before(keyboard),
                )
                    .run_if(in_state(DuelPhase::Playing)),
            )
            .add_systems(OnExit(DuelPhase::Playing), table_settings::put_away)
            // The settings screen's controls read back (WP5).
            .add_systems(
                Update,
                (
                    crate::settingsui::keys::mirror_in_use,
                    crate::settingsui::keys::slider_keys,
                    crate::settingsui::keys::apply_sliders,
                    crate::settingsui::keys::follow_nav,
                    crate::settingsui::keys::watch_trial,
                    crate::settingsui::keys::copy_out,
                )
                    .chain()
                    .after(crate::shellkit::focus::FocusSystems)
                    .run_if(table_settings::lobby_or_over_table),
            )
            // The shell keymap: the lobby says what is open, and answers
            // the screen moves a shell key asks for (`shortcuts`).
            .add_systems(
                Update,
                (
                    (
                        shortcuts::write_stack.in_set(crate::shellkit::keys::StackSystems),
                        // After the lobby's own typing, so the key that moves
                        // the caret into a field is not typed into it (`/`).
                        // And before the tree is drawn, so what a key opened
                        // (the builder's Export) is on screen that frame;
                        // unordered, it was sometimes a frame late.
                        shortcuts::run_fired
                            .after(crate::shellkit::keys::KeySystems)
                            .after(keyboard)
                            .before(ui),
                    )
                        .run_if(in_state(DuelPhase::Closed)),
                    shortcuts::focus_the_settings_search
                        .after(crate::shellkit::keys::KeySystems)
                        .after(crate::shellkit::focus::FocusSystems)
                        .run_if(table_settings::lobby_or_over_table),
                ),
            )
            // The Decks screen's Undo, Play's first-run house list (WP2,
            // WP3), and the menus placed once laid out.
            .add_systems(
                Update,
                (
                    decks::undo_clock,
                    touring::tours
                        .after(crate::tour::TourSystems::Input)
                        .after(poll)
                        .after(clicks)
                        .after(keyboard)
                        .after(shortcuts::run_fired)
                        .after(play::follow_the_room)
                        .before(ui),
                    decks::write_clipboard,
                    #[cfg(not(target_arch = "wasm32"))]
                    decks::paste_or_drop,
                    play::ask_for_house,
                    play::follow_the_room,
                    // The Replays tab reads its lists, and a replay asked
                    // for is opened (after the mailbox, before the tree).
                    (replays::visit, replays::open_replay)
                        .after(poll)
                        .before(ui),
                )
                    .run_if(in_state(DuelPhase::Closed)),
            )
            .add_systems(
                PostUpdate,
                (
                    (
                        menus::place
                            .after(bevy::ui::UiSystems::Layout)
                            .before(bevy::ui::UiSystems::PostLayout),
                        crate::buildui::focus::place_menus
                            .after(bevy::ui::UiSystems::Layout)
                            .before(bevy::ui::UiSystems::PostLayout),
                        crate::buildui::sample_hand::reveal.after(bevy::ui::UiSystems::Layout),
                        memorial::return_focus
                            .after(focusing::follow_focus)
                            .before(crate::shellkit::focus::FocusVisuals),
                    )
                        .run_if(in_state(DuelPhase::Closed)),
                    focusing::follow_focus.run_if(table_settings::lobby_or_over_table),
                ),
            )
            // The keyboard on the stops the screens draw (WP2, WP3): Enter
            // and Space, a tile's own keys, a screen's first focus.
            .add_systems(
                Update,
                (
                    (
                        focusing::keys_press,
                        focusing::initial_focus,
                        focusing::down_from_search,
                    )
                        .run_if(table_settings::lobby_or_over_table),
                    focusing::tile_keys.run_if(in_state(DuelPhase::Closed)),
                )
                    .after(crate::shellkit::focus::FocusSystems)
                    // Before the tree is drawn: a press a key made lands in
                    // this frame's tree, and the focused entity a key reads
                    // is not one a rebuild has just despawned (unordered,
                    // Enter on the Create-table sheet found no focus).
                    .before(ui),
            )
            // In every phase: a table is where most pictures and text are
            // asked for.
            .init_resource::<crate::cardtext::TextGateway>()
            .add_systems(Update, (art_follows_the_session, text_follows_the_session))
            .init_resource::<card_fill::FillBoard>()
            .add_systems(
                Update,
                (
                    card_fill::steer.after(art_follows_the_session),
                    (
                        card_fill::read,
                        card_fill::show.after(card_fill::read).after(ui),
                        card_fill::show_mark.after(card_fill::read).after(ui),
                    )
                        .run_if(in_state(DuelPhase::Closed)),
                ),
            )
            .add_systems(OnEnter(DuelPhase::Closed), (came_back, spawn_camera))
            .init_resource::<Hovered>()
            .add_message::<PointerOver>()
            .add_message::<PointerOut>()
            .add_message::<bevy::picking::events::PointerPress>()
            .add_message::<PointerRelease>()
            .add_message::<bevy::picking::pointer::PointerInput>()
            .add_systems(
                OnExit(DuelPhase::Closed),
                (teardown, despawn_preview, hint::despawn_hint),
            )
            .add_systems(
                Update,
                spawn_leave_button.run_if(in_state(DuelPhase::Finished)),
            )
            .add_systems(OnExit(DuelPhase::Finished), despawn_leave_button);
    }
}

/// The deck builder's systems beside the lobby's: Enter on a focused
/// control, its keyboard place and the kit's focus as one, its draft, its
/// save state — after the kit's focus and the lobby's keys, before the tree
/// is drawn.
fn install_builder(app: &mut App) {
    app.init_resource::<crate::buildui::virtual_rows::ListProbe>()
        .add_systems(
            Update,
            (
                build_press::activate_focused,
                crate::buildui::focus::follow,
                crate::buildui::draft::keep_the_draft,
                crate::buildui::draft::follow_the_save,
            )
                .chain()
                .after(crate::shellkit::focus::FocusSystems)
                .after(keyboard)
                .before(ui)
                .run_if(in_state(DuelPhase::Closed)),
        )
        // The printing picker's card, where its picture never comes: after
        // the frame's tree is drawn, and in a schedule of its own so that it
        // moves no system in `Update`.
        .add_systems(
            PostUpdate,
            crate::buildui::print_picker::face_lost_art.run_if(in_state(DuelPhase::Closed)),
        );
}

// ------------------------------------------------------------- resources
