//! The retained HUD tree: the prompt slip, the stack, the tray, and the one
//! system that rebuilds all of it.
//!
//! Rebuilt only when [`HudRevision`] says something it draws has changed —
//! a rebuild per frame would cost more than the whole table does.
//!
//! What a seat *is* — its name, its life, its zones, the turn it is taking —
//! is no longer drawn here. That is [`super::seatbar`], written on each
//! seat's own mat, and the tab strip and the phase rail this module used to
//! open with are gone with it.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::commanderdamage;
use baylee_client_core::interaction::Prompt;

/// How wide the commander-damage track is drawn, in logical pixels.
///
/// The same for every seat, whatever that seat's worst source is, because the
/// bar's whole job is to be comparable: it stands for twenty-one
/// (`commanderdamage::LETHAL`) and nothing else, so a half-full bar means the
/// same thing under every name at the table.
#[expect(dead_code, reason = "the seat sheet draws the track next")]
const TRACK_W: f32 = 52.0;

/// The gap between two answers on the prompt slip, in logical pixels.
///
/// Small, and it has to be: the answers divide the sheet between them, so
/// every pixel here is a pixel off each button. Wide enough that two brass
/// edges never touch, narrow enough that the row still reads as one control
/// with parts rather than as scattered buttons.
pub(super) const BUTTON_GAP: f32 = 8.0;

/// The label on an answer, in logical pixels.
pub(super) const ANSWER_PT: f32 = 13.0;

/// The caption over the card underneath a copy, one line of nine-pixel type.
///
/// Named because it is the difference between bottom-aligning the *pair* with
/// the preview and bottom-aligning the card alone, which would leave the two
/// feet a caption apart.
const CAPTION_H: f32 = 13.0;

/// Removes the overlay when the duel hands the screen back.
///
/// The 3D stage has always been torn down on `Close`; the overlay was not,
/// because until the client grew a lobby nothing ever closed a duel and came
/// back to something else. The revision goes with it: it describes a tree that
/// no longer exists, and the next duel's first frame has to rebuild rather
/// than compare against it.
pub fn despawn_overlay(
    mut commands: Commands,
    existing: Query<Entity, With<HudRoot>>,
    mut revision: ResMut<HudRevision>,
    mut ledge: ResMut<ledge::LedgeRevision>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    *revision = HudRevision::default();
    // The shelf goes with the root it hangs off, so its own counter describes
    // a tree that is not there either. Same reason, one level down.
    *ledge = ledge::LedgeRevision::default();
    // The cache is what holds those materials alive, so letting go of it here
    // is what actually frees them: a duel that ended must not leave a hand's
    // worth behind for the next one.
    if let Some(mut cache) = ui_materials {
        cache.clear();
    }
}

/// Everything this bar paints *with* that only exists when there is a render
/// world to paint in.
///
/// Three things travel as one because they are one answer to the same
/// question, and because `sync_overlay` is a system with sixteen parameters
/// and bevy implements `SystemParam` for tuples no longer than that — the
/// seventeenth is not a compile error about the limit, it is "`sync_overlay`
/// is not a system set" at every `.after()` in `lib.rs`, which is a long way
/// from the cause.
#[derive(bevy::ecs::system::SystemParam)]
pub struct Surfaces<'w> {
    /// The parchment, generated at startup — a headless app that never ran
    /// `setup_sheets` simply draws the flat colour the sheet is grained
    /// around.
    sheets: Option<Res<'w, UiSheets>>,
    /// The cloth: two handles, each minted on the first frame there is
    /// somewhere to mint it and cloned on every frame after — the skirt over
    /// the whole hand zone, the rail over the actions row.
    ///
    /// Optional like the rest of this bundle, and for the same reason twice
    /// over: `FrontalPlugin` is what puts it there, and a test app that
    /// builds this tree adds no plugins at all. A required resource here is
    /// not a missing cloth — it is every overlay test panicking inside the
    /// scheduler, with the parameter's name switched off unless the `debug`
    /// feature is on.
    cloth: Option<ResMut<'w, crate::frontal::Cloth>>,
    /// Where they are minted. Absent headless, and then there is no cloth and
    /// both surfaces draw the flat dye instead.
    cloth_assets: Option<ResMut<'w, Assets<crate::frontal::FrontalMaterial>>>,
}

impl Surfaces<'_> {
    /// The skirt's handle, or `None` when there is nowhere to draw it.
    fn skirt(&mut self) -> Option<Handle<crate::frontal::FrontalMaterial>> {
        let assets = self.cloth_assets.as_deref_mut();
        self.cloth.as_mut()?.skirt(assets)
    }

    /// The rail's, which is the same cloth cut for a node 40 pixels tall.
    fn rail(&mut self) -> Option<Handle<crate::frontal::FrontalMaterial>> {
        let assets = self.cloth_assets.as_deref_mut();
        self.cloth.as_mut()?.rail(assets)
    }
}

/// Rebuilds the overlay when anything it shows changes.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)] // one retained-UI rebuild, sectioned by comments
pub fn sync_overlay(
    mut commands: Commands,
    duel: Res<Duel>,
    mut revision: ResMut<HudRevision>,
    tree: OverlayTree,
    mut textures: ResMut<CardTextures>,
    assets: Res<AssetServer>,
    windows: Query<&Window>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    prefs: Res<crate::prefs::Prefs>,
    texts: Res<crate::cardtext::CardTexts>,
    mode: Res<crate::face::FaceMode>,
    motion: super::CardMotion,
    // Both come from the render plugins. A headless app has neither, and
    // every card below falls back to a plain image rather than growing a
    // second code path for it.
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
    mut surfaces: Surfaces,
) {
    let skirt = surfaces.skirt();
    let rail = surfaces.rail();
    let mut cards = match (ui_materials, material_assets) {
        (Some(cache), Some(assets)) => Some((cache, assets)),
        _ => None,
    };
    let mut cards = cards.as_mut().map(|(cache, assets)| UiCards {
        cache: cache.as_mut(),
        assets: assets.as_mut(),
    });
    let faces = FaceCtx {
        texts: &texts,
        mode: &mode,
        settings: &settings,
        view: duel.view.as_ref(),
    };
    let lang = Lang::of(&settings.lang);
    let seq = duel.board.as_ref().map(|b| b.seq);
    // A finished game is the one question this bar does not answer. Who won
    // is the end screen's whole subject, set at four times this size; a
    // dimmed second copy of it under the screen's own would be the same
    // sentence twice, and `Prompt::GameOver`'s headline ("The game is over")
    // would be a third wording of it beside the other two. So the bar simply
    // stops: there is no question left for a question bar to hold.
    //
    // It stops **whole**, and that is the part worth stating. The slip is
    // drawn for a question *or* a refusal *or* a word about the connection,
    // so silencing only the question leaves two ways for the bar to come
    // back under the end screen — and both are answers to a game that is
    // still being played. A refusal is the engine turning down an action,
    // and there are no actions left (`DuelSet::Input` does not run in
    // `Finished`). A word about the connection is a table waiting for you,
    // and this one has stopped waiting: the gateway drops the socket after
    // `GameEnded`, and a red "the connection to the table was lost" under
    // "You won" would be reporting a loss that cost the player nothing.
    let over = duel.ending().is_some();
    // Whose turn it is, for the one line that changes with it. A seat holds
    // priority on every turn at the table, so the bar has to be told which
    // one this is or it says "Your move" through the whole game.
    let turn = duel
        .view
        .as_ref()
        .map_or(baylee_client_core::Turn::Mine, |v| {
            baylee_client_core::Turn::of(v.active, v.seat)
        });
    // The client's own cast chooser speaks in the bar's own voice while it
    // stands. The engine is holding an ordinary priority window behind it —
    // which is exactly the window this client has to ask *inside*, because
    // the engine cannot count a spell's ways until the mana is floating and
    // this is the thing that floats it. See [`crate::CastMenu`].
    let cast_menu = duel
        .cast_menu
        .as_ref()
        .filter(|_| !over)
        .map(|m| (m.card, m.modes.len(), m.pick));
    let prompt = duel
        .cast_menu
        .as_ref()
        .filter(|_| !over)
        .map(|m| {
            m.prompt().headline(
                lang,
                turn,
                duel.statics.as_ref(),
                duel.view.as_ref().is_some_and(|v| v.owed.is_some()),
            )
        })
        .or_else(|| {
            duel.interaction.as_ref().filter(|_| !over).map(|i| {
                i.prompt().headline(
                    lang,
                    turn,
                    duel.statics.as_ref(),
                    duel.view.as_ref().is_some_and(|v| v.owed.is_some()),
                )
            })
        });
    // A refusal used to *stand in* for the headline, which meant it was only
    // ever seen when nothing was being asked — and the engine refuses an
    // answer precisely while a question is standing. The player clicked, the
    // bar went on saying "Choose a target", and nothing else happened. It is
    // its own line now, under whatever the bar was already saying.
    let error = duel.last_error.clone().filter(|_| !over);
    // The connection, when it has something to say. Drawn in the same bar
    // rather than in a banner of its own because that is where this client
    // already speaks to the player, and above the rest of it because a table
    // that cannot hear you makes every other line on the bar moot.
    let link_note = duel.link_note.filter(|_| !over);
    let hovered = duel.hovered;
    let selected: Vec<ObjectId> = duel
        .interaction
        .as_ref()
        .map(|i| i.selected().collect::<Vec<_>>())
        .unwrap_or_default();
    let orders = prefs.orders().clone();
    let autopilot = duel.autopilot;
    let combat = duel.interaction.as_ref().and_then(|i| {
        i.focus_position()
            .map(|(focus, count)| (focus, count, i.declared()))
    });
    let focus = duel.focus;
    let preview_scale = settings.preview_scale;
    let armed_deed = duel.armed.clone();
    // Rounded to whole pixels: a window being dragged reports fractional
    // sizes, and a revision keyed on an `f32` would rebuild the whole tree on
    // a sub-pixel wobble.
    #[allow(clippy::cast_possible_truncation)]
    let canvas = windows
        .single()
        .map_or((1200, 800), |w| (w.width() as i32, w.height() as i32));
    let number = duel
        .interaction
        .as_ref()
        .and_then(|i| matches!(i.prompt(), Prompt::ChooseNumber { .. }).then(|| i.number()));
    let choice = duel
        .interaction
        .as_ref()
        .and_then(baylee_client_core::Interaction::chosen_index);

    if revision.seq == seq
        && revision.hand_order == duel.hand_order
        && revision.prompt == prompt
        && revision.error == error
        && revision.link_note == link_note
        && revision.hovered == hovered
        && revision.selected == selected
        && revision.orders.as_ref().is_some_and(|o| o.same_as(&orders))
        && revision.autopilot == autopilot
        && revision.focus == focus
        && (revision.preview_scale - preview_scale).abs() < f32::EPSILON
        && revision.faces == faces.always()
        && revision.texts == texts.len()
        && revision.arrivals == textures.epoch()
        && revision.combat == combat
        && revision.armed == armed_deed
        && revision.number == number
        && revision.choice == choice
        && revision.cast_menu == cast_menu
        && revision.window == canvas
        && !tree.root.is_empty()
    {
        return;
    }
    revision.seq = seq;
    revision.hand_order = duel.hand_order;
    revision.prompt.clone_from(&prompt);
    revision.error.clone_from(&error);
    revision.link_note = link_note;
    revision.hovered = hovered;
    revision.selected.clone_from(&selected);
    revision.orders = Some(orders.clone());
    revision.autopilot = autopilot;
    revision.focus = focus;
    revision.preview_scale = preview_scale;
    revision.faces = faces.always();
    revision.texts = texts.len();
    revision.arrivals = textures.epoch();
    revision.combat = combat;
    revision.armed.clone_from(&armed_deed);
    revision.number = number;
    revision.choice = choice;
    revision.cast_menu = cast_menu;
    revision.window = canvas;

    let (Some(board), Some(view)) = (duel.board.as_ref(), duel.view.as_ref()) else {
        // Nothing to draw yet, or nothing left: the tree describes a game
        // that is not there, so all of it goes — the shelf included. The
        // guard above then finds no root and rebuilds from the first frame
        // there is one, which is what this early return has always bought.
        for (root, _) in &tree.root {
            commands.entity(root).despawn();
        }
        return;
    };

    // Which of the objects the *overlay* draws this choice will actually
    // accept. `selected` says what a player has picked; this says what they
    // may pick, which is the thing the hand had no way of showing: a cleanup
    // discard lit nothing up at all, so the only clue that the hand was
    // clickable was clicking it.
    //
    // The stack joined it by the same road. Every card in the game that says
    // "target spell" points at a row of that panel, and until those rows
    // became clickable there was nothing there to light.
    let selectable: Vec<ObjectId> = duel
        .interaction
        .as_ref()
        .map(|i| {
            board
                .hand
                .iter()
                .map(|c| c.id)
                .chain(board.stack.iter().map(|item| item.id))
                .filter(|id| i.is_selectable(*id))
                .collect()
        })
        .unwrap_or_default();

    // The rebuild, which is no longer a clean sweep: the root and the shelf
    // stand, everything else is torn down and written again.
    //
    // The shelf is the exception because it is the one thing here that is not
    // a picture of the snapshot — it is the edge the window ends at, and the
    // buttons on it hold a `Feel` whose warmth would be lost every time the
    // pointer crossed a card. `ledge::LedgeShelf` has the whole argument,
    // including why it cannot simply be a second root the way the seat bars
    // are.
    //
    // The design (§10.2 step 1) put the shelf in this tree and (§6) asked for
    // a revision counter of its own in the same breath, which cannot both be
    // true while a rebuild despawns the root: a counter governing a subtree
    // that is deleted on every pointer move governs nothing. Keeping the root
    // is the smaller of the two ways out — the other moves four `ZIndex`
    // layers to `GlobalZIndex` to make room for a ledge root between the veil
    // and the preview.
    let root = if let Ok((root, children)) = tree.root.single() {
        for child in children.into_iter().flatten() {
            if !tree.shelf.contains(*child)
                && !tree.drawer.contains(*child)
                && !tree.veil.contains(*child)
                && !tree.panel.contains(*child)
                && !tree.tray.contains(*child)
                && !tree.pool.contains(*child)
                && !tree.menu.contains(*child)
            {
                commands.entity(*child).despawn();
            }
        }
        root
    } else {
        let root = commands
            .spawn((
                HudRoot,
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                // The overlay must never eat clicks meant for the table.
                Pickable::IGNORE,
            ))
            .id();
        // The ledge stands on top of the hand zone and is a *sibling* of it,
        // not a child: `ZIndex` counts among siblings, and the question has
        // to stand over the table veil the zone dialog paints while the hand
        // goes dark under it. `ledge::spawn_ledge` has the whole reason.
        //
        // Spawned with the root rather than with the hand below it, because
        // it outlives every rebuild the hand does not, and because it is
        // drawn whether or not there is a hand to draw: §6's first principle
        // is that the zone's height never changes.
        let ledge = ledge::spawn_ledge(&mut commands, rail);
        commands.entity(root).add_child(ledge);
        // The drawer's node, which outlives every
        // rebuild this system does, and what fills it is not any of this
        // system's business. See [`ledge::drawer`].
        let drawer = ledge::drawer::spawn_drawer_root(&mut commands);
        commands.entity(root).add_child(drawer);
        // And the tray, for the third time and the same two reasons. It is
        // the one of the three whose *whole point* is outliving this rebuild:
        // the shelf's own children come and go with every sentence, and the
        // owner asked for a button that is always there. See [`ledge::tray`].
        let tray = ledge::tray::spawn_tray_strip(&mut commands);
        commands.entity(root).add_child(tray);
        // And the mana pool, which is the tray's mirror down to the node the
        // two of them spawn — it hangs off the shelf's left end instead. It
        // was a retained *column on* the shelf until the owner asked for the
        // symmetry; it has always had to outlive this rebuild, because a mana
        // arriving is drawn arriving and a question changes under it.
        // See [`ledge::pool`].
        let pool = ledge::pool::spawn_pool_strip(&mut commands);
        commands.entity(root).add_child(pool);
        // And the game menu's panel, which is the fifth and the only one that
        // is usually not on the screen at all. What it has to outlive is not
        // this rebuild but the *shelf's*: arming a concession changes
        // `LedgeRevision`, which despawns the shelf's columns, and the panel
        // is where the second press has to be made. See [`ledge::menu`].
        let menu = ledge::menu::spawn_menu_panel(&mut commands);
        commands.entity(root).add_child(menu);
        root
    };

    // The question and everything it needs are no longer drawn here. The four
    // things that are one line each went to the shelf in §10.2 step 3, and in
    // step 6 the rest followed them into the drawer: the pick hint, combat's
    // aim and its threat, the number stepper, the subtype filter and the
    // indexed chooser. The slip that carried them is gone with them; what
    // still uses this sheet is the hover preview, further down.

    // ---- bottom: the hand zone (always on top) ---------------------------
    if let Some(statics) = duel.statics.as_ref() {
        let available = windows
            .single()
            .map_or(1200.0, |w| hand_available(w.width()));
        let layout = grouped_hand_layout(board.hand.len(), available, &duel.hand_groups);
        let hand_zone = spawn_hand_zone(
            &mut commands,
            lang,
            board,
            view,
            statics,
            hovered,
            &selected,
            &selectable,
            duel.armed.as_ref(),
            layout,
            duel.hand_scroll,
            duel.hand_order,
            &duel.hand_groups,
            &mut textures,
            &assets,
            &fonts,
            &faces,
            &motion.sheen,
            &motion.touch,
            cards.as_mut(),
            skirt,
        );
        commands.entity(root).add_child(hand_zone);

        // ---- card preview: a speech-bubble tooltip over the hovered
        // card (hand, own battlefield, or command zone). No title text —
        // the image is big enough to read.
        if let Some((art, anchor)) = preview_anchor(
            board,
            view,
            hovered,
            layout,
            duel.hand_scroll,
            duel.hovered_at,
        ) {
            let scale = settings.preview_scale.clamp(0.5, 1.75);
            let window = windows.single().map_or(Vec2::new(1200.0, 800.0), |w| {
                Vec2::new(w.width(), w.height())
            });
            // What the scale slider asked for, and then what this window can
            // actually show: a preview larger than the screen is cut off
            // wherever it is placed, and no amount of arithmetic in
            // `preview_place` can rescue it. The slider is a preference; the
            // window is not.
            let want = Vec2::new(308.0 * scale, 308.0 * scale * 88.0 / 63.0);
            // The sentence the stack is abbreviating, when that is what the
            // pointer is on. It is part of the bubble's *size* and therefore
            // has to be resolved before the picture is: a card sized to the
            // window and a sheet hung under it would put the sheet off the
            // bottom of the screen. See `super::slip`.
            let slip = super::slip::says(board, view, statics, lang, &faces, hovered);
            let runs = slip.as_ref().map(super::slip::runs).unwrap_or_default();
            // Twice, at two widths, and the first one is the estimate that
            // decides how much room the picture may take. The second is at
            // the width the sheet will actually be drawn at, which is the one
            // the placement and the resize handle are measured from.
            let asked = slip.as_ref().map_or(0.0, |s| {
                super::slip::height(&runs, want.x, s.kind.is_some())
            });
            let art_size = preview_art_size(want, 6.0, window - Vec2::new(0.0, asked));
            let (img_w, img_h) = (art_size.x, art_size.y);
            let slip_h = slip
                .as_ref()
                .map_or(0.0, |s| super::slip::height(&runs, img_w, s.kind.is_some()));
            // The panel is the picture plus its six pixels of padding on
            // every side, and the sheet under it when there is one.
            let panel = art_size + Vec2::splat(12.0) + Vec2::new(0.0, slip_h);
            // The drawer, if one is open, as the rectangle the preview
            // must not cover. Read off the tree rather than rebuilt from
            // `drawer.rs`'s constants: what is drawn is what a player sees
            // covered, and the panel is scaled while it opens.
            //
            // `UiGlobalTransform` is in physical pixels and centred on the
            // node, which is why both halves of this are converted and the
            // size is halved before it is spent.
            let keep_out = tree.drawer_box.iter().next().map(|(computed, at)| {
                let size = computed.size() * computed.inverse_scale_factor;
                let centre = at.translation * computed.inverse_scale_factor;
                Rect::from_center_size(centre, size)
            });
            let place = preview_place(anchor, panel, window, keep_out);
            let key = art.map(|art| ImageKey {
                size: ArtSize::Normal,
                ..art
            });
            // What can actually be drawn *this frame*, which is not always
            // what the preview asks for. This is the only place that wants
            // `Normal`, and `Preload` warms that size for the hand and the
            // local command zone alone — a battlefield has no bound, and
            // eighty permanents at 1.3 MB apiece would spend the whole browser
            // budget on a convenience. So hovering an opponent's permanent
            // arrives here with nothing at this size every single time.
            //
            // What filled the gap was the *constructed face*: a text card,
            // reading "Rules text unavailable" wherever the catalog is not
            // wired up, for as long as the fetch takes. The board's own
            // `Small` art is already resident for anything on the table, and
            // stretching 146 pixels to 308 is soft for half a second — but it
            // is a picture of the card, and the other thing is not.
            let small = art.map(|art| ImageKey {
                size: ArtSize::Small,
                ..art
            });
            let stopgap = match (key, small) {
                (Some(big), Some(small))
                    if !textures.has_arrived(big) && textures.has_arrived(small) =>
                {
                    Some(small)
                }
                _ => None,
            };
            let shown = stopgap.or(key);
            // The face first: it only borrows the cache, and the image below
            // needs it mutably. Asked about `shown`, so the stopgap counts as
            // art having arrived and the text card stays off the screen.
            let built = hovered.and_then(|id| preview_face(&faces, view, &textures, id, shown));
            let image = match key {
                // Always ask for the full-size art, even when the stopgap is
                // what gets drawn — asking is what starts the fetch, and a
                // preview that settled for `Small` would never sharpen.
                Some(key) => {
                    let full = textures.get(key, statics, &assets);
                    match stopgap {
                        Some(small) => textures.get(small, statics, &assets),
                        None => full,
                    }
                }
                None => textures.card_back(),
            };
            let visual = spawn_card_art(
                &mut commands,
                lang,
                image,
                built.as_ref(),
                img_w,
                img_h,
                crate::face::Detail::Full,
                &fonts,
                {
                    // The preview is the same permanent drawn larger, so it
                    // says the same numbers: a 2/2 under an anthem is a 3/3
                    // on the table, and a preview showing the printed 2/2
                    // would put two answers for one creature on one screen.
                    // A card in hand has no view object and therefore no
                    // corner, which is right — its printed body is what it is.
                    let object = hovered.and_then(|id| view.object(id));
                    let corner = object.map_or_else(
                        baylee_client_core::cardplate::Corner::default,
                        baylee_client_core::cardplate::Corner::of_object,
                    );
                    // `shown`, not `key`: the look is the cache key for the
                    // material, and one naming the full-size art while the
                    // handle beside it holds the stopgap would hand the same
                    // material two different textures on consecutive frames.
                    // The sweep is the preview's own, keyed on the panel
                    // having *opened*: the permanent may have been on the
                    // table since turn one, and the thing that is new is the
                    // player looking at it.
                    let sweep =
                        hovered.and_then(|id| motion.sheen.of(id, crate::sheen::Surface::Preview));
                    match shown {
                        Some(shown) => CardLook::art(
                            shown,
                            finish_of(statics, Some(shown)),
                            crate::cardmat::glow_of(object, crate::cardmat::Offer::NONE),
                        )
                        .with_corner(corner)
                        .with_sweep(sweep),
                        None => CardLook::back(FinishTreatment::Plain, 0)
                            .with_corner(corner)
                            .with_sweep(sweep),
                    }
                },
                cards.as_mut(),
            );
            let tooltip = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(place.y),
                        left: px(place.x),
                        padding: UiRect::all(px(6)),
                        // A column, because the bubble is a card and — when
                        // the pointer is on the stack — a sheet under it.
                        flex_direction: FlexDirection::Column,
                        border_radius: preview_radius(img_w),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    // Transparent, like the hand zone under it and for the
                    // same reason: the preview is a *card* held up to the
                    // light, and it was drawn as a card inside a dark tile
                    // six pixels bigger on every side. The padding stays —
                    // it is the gap the shadow needs in order to read as a
                    // shadow rather than as a rim — and the card's own alpha
                    // cut keeps the scan's white corners off the screen.
                    BackgroundColor(Color::NONE),
                    upward_shadow(),
                    ZIndex(Z_PREVIEW),
                    Pickable::IGNORE,
                    children![(
                        // Resize handle, bottom right — of the *card*, which
                        // is not the bottom of the bubble once a sheet hangs
                        // under it. It resizes the picture, so it stays on
                        // the picture's corner rather than landing in the
                        // middle of a sentence.
                        PreviewResize,
                        Node {
                            position_type: PositionType::Absolute,
                            right: px(4),
                            bottom: px(4.0 + slip_h),
                            padding: UiRect::all(px(4)),
                            border_radius: btn_radius(),
                            ..default()
                        },
                        BackgroundColor(palette::PANEL),
                        children![(
                            Text::new(glyph::EXPAND.to_string()),
                            icon_tf(&fonts, 11.0),
                            TextColor(palette::MUTED),
                        )],
                    ),],
                ))
                .id();
            // Every card can be turned over with shift. The frame holds both
            // sides and the turn, so the shape of the tree does not depend on
            // the card — what differs is only what is *on* the far side: the
            // second face of a double-faced printing, or the printed back,
            // which is what a card looks like from behind and what everyone
            // else at the table is looking at while you hold it.
            let frame = commands
                .spawn((
                    crate::flip::Flip::default(),
                    Node {
                        width: px(img_w),
                        height: px(img_h),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(visual).insert((
                crate::flip::Side::Front,
                Visibility::Inherited,
                face_node(img_w, img_h),
            ));
            commands.entity(frame).add_child(visual);
            let (art, look) = match far_face(key, has_back_image(view, hovered)) {
                Some(back) => (
                    textures.get(back, statics, &assets),
                    CardLook::art(back, finish_of(statics, Some(back)), 0),
                ),
                // No corner on the back: power, toughness and counters are
                // printed on the face and a card lying face down shows none
                // of them.
                None => (
                    textures.card_back(),
                    CardLook::back(FinishTreatment::Plain, 0),
                ),
            };
            let far = spawn_card_art(
                &mut commands,
                lang,
                art,
                None,
                img_w,
                img_h,
                crate::face::Detail::Full,
                &fonts,
                look,
                cards.as_mut(),
            );
            commands.entity(far).insert((
                crate::flip::Side::Back,
                // Hidden until the turn passes the quarter, where the card is
                // edge-on and the swap cannot be seen.
                Visibility::Hidden,
                face_node(img_w, img_h),
            ));
            commands.entity(frame).add_child(far);
            commands.entity(tooltip).add_child(frame);
            // The preview is a *description of* the hovered card, so it must
            // never take the pointer from it. The frame above already ignores
            // the pointer — but `Pickable` does not inherit, and the card face
            // inside it is a UI node like any other, 308 by 430 of it. A board
            // card's preview is centred on the window, which is exactly where
            // a player's own lands sit, so hovering one dropped a pickable
            // panel over the very pointer that opened it. The card reported
            // `Out` on the next frame, the preview closed, the card reported
            // `Over`, and it blinked — the flicker reported on "my own mana
            // cards". It was found by photographing the table, not by reading
            // this code: an opponent's permanent behaved perfectly, because
            // the panel opens nowhere near the top of the felt.
            //
            // Standing still is the worse half. The grace window in
            // `pointer_hover` expires mid-oscillation, and the `Out` it
            // discards is the only one there will ever be: the card is out of
            // the hover map from then on, so every later `Out` names the panel
            // instead and matches no `CardVisual`. The hover latches on a card
            // the pointer left minutes ago.
            //
            // Recursive, because the face has children of its own and each
            // would take the pointer alone. The resize handle hangs off
            // `tooltip` rather than `frame`, so it stays pickable — which is
            // the whole reason those are two entities.
            commands
                .entity(frame)
                .insert_recursive::<Children>(Pickable::IGNORE);
            // The sheet, under the card and inside the same bubble — so the
            // two move together, are clipped together, and read as one thing
            // held up rather than as a panel that opened beside a panel.
            if let Some(slip) = slip {
                let sheet = super::slip::spawn(
                    &mut commands,
                    &slip,
                    runs,
                    img_w,
                    window.y,
                    &fonts,
                    surfaces.sheets.as_deref(),
                );
                commands.entity(tooltip).add_child(sheet);
            }
            commands.entity(root).add_child(tooltip);

            // ---- the card underneath a copy ---------------------------
            //
            // The preview above draws what this permanent *is*: a Spark
            // Double wearing Llanowar Elves is a Llanowar Elves, corner mark
            // and all. That is backlog item 1, and it spends the one thing
            // the table used to say plainly — which piece of cardboard is
            // actually lying there. The mark says *that* it is a copy; this
            // says of what.
            //
            // It stands beside the preview rather than on the card. The
            // owner asked for it on the card, and that is a second texture
            // binding on `CardMaterial` and both card shaders — a material
            // change, and not one to make silently inside a pass about how
            // the table looks.
            //
            // Resolved from the view here rather than read off the board
            // model, for the same reason `cardmat::glow_of` reaches the
            // registry itself: this block already resolves everything else
            // about the hovered object, and two lookups that could disagree
            // would be two answers for one card. `CardGroup::original` is
            // the same judgement made once more, and it earns its place by
            // putting the picture in `required_images` — a hover has no
            // frame to spend fetching one.
            if let Some(under) = hovered.and_then(|id| view.object(id)).and_then(|o| {
                baylee_client_core::board::original_of(
                    o,
                    ArtSize::Small,
                    crate::cardart::registry(),
                )
            }) {
                let thumb_w = (img_w * 0.34).max(56.0);
                let thumb_h = thumb_w * 88.0 / 63.0;
                let at = underneath_place(
                    Rect::from_corners(place, place + panel),
                    Vec2::new(thumb_w, thumb_h + CAPTION_H),
                    window,
                );
                let image = textures.get(under, statics, &assets);
                let card = spawn_card_art(
                    &mut commands,
                    lang,
                    image,
                    None,
                    thumb_w,
                    thumb_h,
                    crate::face::Detail::Compact,
                    &fonts,
                    // No corner and no glow: power, toughness and counters
                    // belong to the permanent on the table, and the
                    // permanent on the table is the big card. This is a
                    // picture of a printing.
                    CardLook::art(under, finish_of(statics, Some(under)), 0),
                    cards.as_mut(),
                );
                commands.entity(card).insert(upward_shadow());
                let beside = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(at.x),
                            top: px(at.y),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: px(2),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        ZIndex(Z_PREVIEW),
                        Pickable::IGNORE,
                        children![(
                            Text::new(Phrase::CardUnderneath.text(lang).to_string()),
                            tf(&fonts, 9.0),
                            TextColor(palette::MUTED),
                            Pickable::IGNORE,
                        )],
                    ))
                    .id();
                commands.entity(beside).add_child(card);
                // For the reason the preview itself is unpickable, and it is
                // the worse case here: this panel stands *beside* the
                // preview, which on a board card means directly over the
                // neighbouring permanent.
                commands
                    .entity(beside)
                    .insert_recursive::<Children>(Pickable::IGNORE);
                commands.entity(root).add_child(beside);
            }

            // The speech-bubble tail, pointing down at the hovered card —
            // and only for a hand card, which is the only one the tail can
            // point *at*. A panel standing beside a permanent needs none: it
            // is already next to the thing it describes, and a caret aimed
            // down into the hand zone from there would name a card at random.
            if let PreviewAt::Hand(x) = anchor {
                let tail = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: px(HAND_ZONE_H + 2.0),
                            left: px(x - 9.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                        children![(
                            Text::new(glyph::CARET_DOWN.to_string()),
                            icon_tf(&fonts, 18.0),
                            TextColor(palette::PANEL_LIT),
                        )],
                    ))
                    .id();
                commands.entity(root).add_child(tail);
            }
        }
    }

    // ---- the stack (left of the rail, when non-empty) --------------------
    //
    // And not once the game is over, which is the rule the prompt bar keeps
    // six hundred lines above and for the same reason. The owner pressed
    // "concede" with one of Sheoldred's triggers still on the stack, and the
    // end screen came up with that trigger drawn beside it — an entry whose
    // whole job is to say something is about to happen, in a game where
    // nothing will. `Duel::ending` names three readers in its own doc (the
    // veil, the end screen, the prompt bar); this panel is the fourth, and
    // nobody had connected it.
    if let (false, false, Some(statics)) = (
        duel.ending().is_some(),
        board.stack.is_empty(),
        duel.statics.as_ref(),
    ) {
        let stack = spawn_stack_panel(
            &mut commands,
            lang,
            board,
            view,
            statics,
            &super::stack::Picks {
                hovered,
                selected: &selected,
                selectable: &selectable,
            },
            &mut textures,
            &assets,
            &fonts,
            &faces,
            cards.as_mut(),
        );
        commands.entity(root).add_child(stack);
    }

    // ---- the zone browser ------------------------------------------------
    //
    // Drawn here until the owner reported it flickering under the pointer,
    // and the report was about this system rather than about the dialog:
    // `hovered` is in the gate above, so every pointer move that changed
    // which object was under the cursor despawned the whole tree — and the
    // dialog is a hundred rows the pointer moves *across*. It is
    // `tray::sync_tray` now, with `tray::TrayRevision` counting the things
    // the dialog actually draws from, none of which is a hover.
}

/// What an armed deed calls itself: the words, and the mana those words name.
///
/// The two are separate because a cost is **drawn** and not spelled. A `Text`
/// node can hold the characters `{4}{U}{U}`, and that is what the deck
/// builder deliberately does not do — `manaui::spawn_pip` sets each symbol on
/// its own coloured disc, and this row now says the price the same way every
/// other price in this client is said.
pub(super) struct ArmedWords {
    /// The phrase. When [`Self::cost`] is `Some`, its `{0}` marks where the
    /// pips go and the two halves are laid out either side of them; a phrase
    /// with no `{0}` simply takes the pips after its last word.
    pub(super) text: String,
    /// The mana this deed spends, or `None` when it spends none.
    pub(super) cost: Option<baylee_core::mana::ManaCost>,
}

/// What an armed deed calls itself, or `None` when the engine no longer
/// offers it.
///
/// Resolved against the *current* `LegalActions` here as well as at the two
/// places that fire it, which is what stops a row drawn a frame ago from
/// offering something that has since been withdrawn. The row simply
/// disappears; the state itself is cleared by the next key or tap, both of
/// which run the same resolution.
pub(super) fn armed_label(duel: &Duel, lang: Lang, armed: &crate::Armed) -> Option<ArmedWords> {
    match &armed.deed {
        crate::Deed::Play => duel
            .interaction
            .as_ref()
            .and_then(|i| i.play_card(armed.object))
            .map(|_| ArmedWords {
                text: Phrase::ArmedPlay.text(lang).to_string(),
                // The engine offered this cast, so the mana is already
                // floating (`casting::can_cast` checks the pool): there is no
                // price left to quote.
                cost: None,
            }),
        // An ability's label already carries its whole cost as prose —
        // "{T}, Sacrifice this, Pay 1 life" — because most of that cost is
        // not mana and there are no discs for a sacrifice. Drawing pips
        // beside it would say the mana half twice.
        crate::Deed::Ability(action) => super::ability_options(duel, lang, armed.object)?
            .into_iter()
            .find(|o| o.action == *action)
            .map(|o| ArmedWords {
                text: o.label,
                cost: None,
            }),
        // The owner's report: this said "Tap 3, then cast", which is a fact
        // about the client's plan and not about the spell. What a player
        // needs to read before spending a turn's lands is the *price* —
        // `{4}{U}{U}` — so the plan carries the cost it was built for and
        // the row draws it.
        crate::Deed::Run { plan, then } => {
            let offered = match then {
                crate::RunEnd::Cast => Some((&duel.reachable, Phrase::ArmedPayAndCast)),
                crate::RunEnd::Suspend => Some((&duel.suspend_reach, Phrase::ArmedSuspend)),
                // A pour is never armed: [`crate::input::arm_ability`] starts
                // its run on the press that picks the colour, which is the
                // whole of "the card taps once the mana has been chosen". So
                // there is no row to draw here, and no price to quote either
                // — the mana *is* the point, and it costs a tap.
                crate::RunEnd::Float => None,
            };
            offered
                .filter(|(offered, _)| offered.contains(&armed.object))
                .map(|(_, phrase)| ArmedWords {
                    text: phrase.text(lang).to_string(),
                    cost: Some(plan.cost),
                })
        }
        // The engine is already offering the suspend, so the cost is floating
        // and there is nothing left to quote — the same reason `Play` quotes
        // nothing.
        crate::Deed::Suspend => duel
            .interaction
            .as_ref()
            .and_then(|i| i.suspend(armed.object))
            .map(|_| ArmedWords {
                text: Phrase::ArmedSuspendNow.text(lang).to_string(),
                cost: None,
            }),
    }
}

/// One arm of the number stepper.
/// Where the prompt slip stands.
///
/// A full-width row that centres its one child, rather than a panel pinned to
/// a corner. The slip is the one thing on screen that *must* be answered, and
/// it used to sit in the bottom-right — the corner furthest from the two
/// things a player is already looking at, their own board and the hand under
/// it. Centred over the near edge of that board, the question and the cards
/// that answer it are one place to look.
///
/// The row itself is `Pickable::IGNORE` and paints nothing: it spans the
/// window so that its child can be centred in it, and takes no click away
/// from the table it lies over.
/// One answer on the prompt slip.
///
/// `flex_grow: 1` with a `flex_basis` of **zero** is the whole promise: grow
/// alone divides the *slack* left over after the labels, so "Aim next" and
/// "Declare none" would still come out different widths. A basis of zero
/// takes the labels out of the sum, and the row is cut into equal parts.
pub(super) fn answer_node() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        padding: UiRect::axes(px(10), px(7)),
        border: UiRect::all(px(1)),
        border_radius: btn_radius(),
        ..default()
    }
}

/// One answer on a parchment sheet.
///
/// `lead` is the answer the sheet is *for* — keep, confirm, pass, play again
/// — and is the only one drawn in brass; the rest are the same button in the
/// sheet's own colour, because two equally loud answers make a player read
/// both before finding out which one the sheet meant. A lone answer is a lead
/// answer: there is nothing left for it to be quieter than.
///
/// It carries no marker of its own, because two sheets use it and they name
/// their answers differently: the prompt slip adds a [`PromptButton`] with a
/// `PlayerAction` on it, and the end screen's exits carry the lobby's `Press`
/// — a way *out* of a duel is not a move in one. A widget that knew which of
/// those it was would be a widget only one of them could use.
pub(crate) fn answer_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    lead: bool,
) -> Entity {
    let rest = if lead {
        palette::BRASS
    } else {
        palette::SLIP_GHOST
    };
    commands
        .spawn((
            answer_node(),
            BackgroundColor(rest),
            BorderColor::all(if lead {
                palette::BRASS
            } else {
                palette::PARCHMENT_EDGE
            }),
            soft_shadow(),
            // The same component every other button in the client carries:
            // `ambience::feel` leans it towards the pointer, sinks it under a
            // press and owns its `BackgroundColor` from the first frame on.
            Feel::new(rest),
            children![(
                Text::new(label),
                tf_bold(fonts, ANSWER_PT),
                TextColor(if lead {
                    palette::PARCHMENT_INK
                } else {
                    palette::PARCHMENT_SOFT
                }),
                // A label is a `Node`, and a node under the pointer is what
                // the pointer is *over*: without this the button only ever lit
                // up when the pointer was in its padding, and went dead the
                // moment it crossed the word it is named after. Measured, not
                // guessed — hovering the padding moved 155 levels and hovering
                // the word moved none.
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// The second life total (CR 903.10a), drawn as a track: the commander glyph,
/// a fixed-width bar filled to the worst source, one tick per other
/// commander, and the worst number itself.
///
/// Shown like a life total and not like a row of numbers, because the
/// question is "how close am I to dying to this" and not "what do these add
/// up to" — they add up to nothing the rules recognise. `None` when no
/// commander has connected: a bar sitting at zero under every seat every game
/// would be noise, the same rule poison and energy follow.
///
/// Fixed width, and the same width for every seat: the eye learns what full
/// looks like once and then reads every other seat against it. A bar scaled
/// to its own worst source would make two seats with very different problems
/// look identical.
///
/// Every node of it is `Pickable::IGNORE`, and that is load-bearing rather
/// than tidy: anything pickable in front of a control stops
/// `PickingInteraction` at itself, so a `Feel` behind the track would go dead
/// across its whole width — the label finding again, and invisible in an
/// ordinary game for the same reason the overflow was.
///
/// It was written inside the seat tab and outlived it. It was lifted out
/// **before** the tab was deleted rather than after, which is the whole point
/// of it existing unused for one commit: the seat sheet is to redraw the
/// track exactly as the tab drew it, and "exactly" is not something that can
/// be recovered from a diff two commits later.
#[expect(dead_code, reason = "the seat sheet calls it next")]
pub(super) fn spawn_commander_track(
    commands: &mut Commands,
    fonts: &UiFonts,
    seat: &baylee_view::SeatView,
    muted: Color,
) -> Option<Entity> {
    let track = commanderdamage::Track::of(&seat.commander_damage)?;
    let color = if track.danger { palette::DANGER } else { muted };
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4.0),
                ..default()
            },
            Pickable::IGNORE,
            children![(
                Text::new(glyph::COMMAND.to_string()),
                icon_tf(fonts, 10.0),
                TextColor(color),
                Pickable::IGNORE,
            )],
        ))
        .id();

    let bar = commands
        .spawn((
            Node {
                width: px(TRACK_W),
                height: px(4.0),
                border_radius: BorderRadius::all(px(2.0)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(bar);

    let fill = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                top: px(0.0),
                width: px(TRACK_W * track.fill),
                height: px(4.0),
                border_radius: BorderRadius::all(px(2.0)),
                ..default()
            },
            BackgroundColor(color),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(fill);

    // One tick per other commander, on the same scale. They say how many more
    // clocks are running and how far along each is; which commander is which
    // is the tooltip's answer, the same half this panel already gives for a
    // counter's colour.
    for at in &track.ticks {
        let tick = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(TRACK_W * at),
                    top: px(0.0),
                    width: px(1.0),
                    height: px(4.0),
                    ..default()
                },
                BackgroundColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(tick);
    }

    let worst = commands
        .spawn((
            Text::new(track.worst.to_string()),
            tf(fonts, 11.0),
            TextColor(color),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(worst);
    Some(row)
}

/// What the preview shows once it has been turned over.
///
/// `Some` is the printing's second face; `None` means the printed back —
/// which is the answer for every ordinary card, and the reason shift now
/// turns anything at all. It used to turn only a double-faced card, so the
/// gesture did nothing on nine cards out of ten and read as broken rather
/// than as inapplicable.
///
/// `has_back` is asked separately because having a *key* says nothing about
/// whether the printing has a second picture: every printing has one key, and
/// a card the seat may not see has none while still being turnable — over to
/// the back, which is exactly what everyone else at the table is looking at.
pub(super) const fn far_face(key: Option<ImageKey>, has_back: bool) -> Option<ImageKey> {
    match key {
        Some(key) if has_back => Some(ImageKey {
            face: baylee_client_core::images::Face::Back,
            ..key
        }),
        _ => None,
    }
}

/// Where one face of the preview sits inside the frame that turns it.
///
/// Both faces stand in the same place, one on top of the other, and that is
/// the whole of it — but it has to be said, because a UI node laid out in its
/// parent's flow is an *item in a row*. Two of them in a frame one card wide
/// and taffy shrinks each to half of it: every preview in the client drew its
/// card squeezed into the left half of the panel, with the hidden face's
/// empty slot beside it. `Visibility::Hidden` does not give a node's place
/// back — only `Display::None` does, and a face that left the layout would
/// resize the frame halfway through the turn.
pub(super) fn face_node(width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(0),
        top: px(0),
        width: px(width),
        height: px(height),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// Whether the hovered object has a second picture to turn over to.
///
/// The view says which face is up, not whether there is another one, so the
/// answer comes from the registry the client already links for ability labels
/// and mana sources. A token or a face-down permanent has no card and
/// therefore no back.
///
/// It used to count the card's *compiled* faces, which is a third question
/// and answered neither (#115): an Adventure prints two names on one piece of
/// card, so this returned `true` for nine of them and the overlay then asked
/// Scryfall's `back` shelf for a picture that answers 404. The registry's
/// `sides` table is read off the printing instead, and the same table serves
/// the deck builder — one answer, computed once, rather than this predicate
/// and `PoolCard`'s disagreeing in two crates.
///
/// This doc had been sitting six items further up, above a `pick_hint` that
/// had its own, since whichever splice put it there — the shape
/// `doc-comment-splice-beheads-the-next-item` is named for. It came back when
/// `pick_hint` went to the drawer and left it standing over nothing.
fn has_back_image(view: &PlayerView, hovered: Option<ObjectId>) -> bool {
    hovered
        .and_then(|id| view.object(id))
        .and_then(|object| object.card.as_ref())
        .is_some_and(|card| baylee_cards::sides::has_back_image(card.index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;
    use baylee_core::mana::ManaCost;
    use baylee_engine::win::{EndReason, GameResult, Victor};

    /// Fonts with no asset server behind them: what is under test is which
    /// lines the bar builds, and none of that is the GPU's.
    fn fonts() -> UiFonts {
        UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            bold: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        }
    }

    /// A view holding one permanent, which is the named card.
    ///
    /// `test_support::printed` keys the identity off a `u16` print number;
    /// the registry index is written over it afterwards, because what this
    /// asks about is the *card* and a made-up index would answer `false`
    /// however the predicate was written.
    fn hovering(name: &str) -> (PlayerView, Option<ObjectId>) {
        let index = baylee_cards::decks::by_name(name).expect("in the pool");
        let mut object = baylee_client_core::test_support::printed(1, 0, name, 1);
        object.card.as_mut().expect("printed gives it a card").index = index;
        let id = object.id;
        let view = baylee_client_core::test_support::ViewBuilder::new(2)
            .with_battlefield(0, [object])
            .build();
        (view, Some(id))
    }

    /// The overlay offers a back only for a card that has a second picture.
    ///
    /// This predicate is the **second** implementation of a question
    /// `PoolCard` also answers, and it counted the card's compiled faces
    /// until #115 — so an Adventure, which prints two names on one piece of
    /// card, was handed a back `ImageKey` and the shelf it points at answers
    /// 404. Both now read one table in `baylee_cards::sides`; asking both
    /// directions here is what would catch them coming apart again.
    #[test]
    fn only_a_card_with_a_second_picture_is_turned_over() {
        let (view, id) = hovering("Agadeem's Awakening");
        assert!(
            has_back_image(&view, id),
            "a modal double-faced card has a back to turn to"
        );
        let (view, id) = hovering("Murderous Rider");
        let adventure = baylee_cards::decks::by_name("Murderous Rider").expect("in the pool");
        assert!(
            baylee_cards::by_index(adventure).is_some_and(|def| def.faces.len() > 1),
            "the premise: this card compiles two faces, which is what used to decide"
        );
        assert!(
            !has_back_image(&view, id),
            "an Adventure prints both halves on one side"
        );
        let (view, id) = hovering("Lightning Bolt");
        assert!(!has_back_image(&view, id), "an ordinary card has no back");
    }

    /// Nothing hovered, and a token, are both "no back" rather than a panic.
    #[test]
    fn a_token_and_an_empty_hover_have_no_back() {
        let (view, _) = hovering("Agadeem's Awakening");
        assert!(!has_back_image(&view, None), "nothing is hovered");
        let token = baylee_client_core::test_support::token(7, 0, "Goblin", 1, 1);
        let id = token.id;
        let view = baylee_client_core::test_support::ViewBuilder::new(2)
            .with_battlefield(0, [token])
            .build();
        assert!(!has_back_image(&view, Some(id)), "a token has no card");
    }

    /// A headless app that really runs [`sync_overlay`].
    ///
    /// Not a source-reading test, because the claim is about what the system
    /// *builds* rather than about a component only a renderer creates. The
    /// two optional material resources are left out on purpose — that is the
    /// branch a machine with no GPU takes, and it is the branch that draws
    /// the prose this test reads.
    fn bar_of(duel: Duel) -> App {
        // The stack panel asks for pictures, and an `AssetServer::load`
        // spawns on the IO pool and panics without it. Idempotent, so this
        // and `overlay_with` may both ask; here because a duel with a stack
        // reaches that panel from this harness too.
        bevy::tasks::IoTaskPool::get_or_init(Default::default);
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<Image>();
        let textures = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            CardTextures::new(&mut images, 1 << 20)
        };
        // Motion off, and a clock present at all. The drawer's way out is a
        // movement now, so "shut" means "gone" only once that movement has
        // run — and a bare `App` has no `Time`, so without both of these a
        // dismissed panel would sit at `t = 0` for ever and anything counting
        // panels would be counting one the question had already left. It is
        // the end of the movement and not its absence: see `hud::motion`.
        //
        // The zone dialog joined that on 19.09.2026, when it gained a flight
        // into the tray. Its despawn is `reveal_tray`'s, at the end of the
        // movement, so a harness without both of these would count a sheet
        // for ever after the browser was shut — and with `reduce_motion` the
        // flight is over on the frame it starts, which is what keeps every
        // test written before it meaning what it meant.
        let mut prefs = crate::prefs::Prefs::default();
        prefs.edit().reduce_motion = true;
        app.insert_resource(textures)
            .insert_resource(duel)
            .insert_resource(fonts())
            .insert_resource(crate::settings::ClientSettings::default())
            .insert_resource(prefs)
            .init_resource::<Time>()
            .init_resource::<HudRevision>()
            .init_resource::<crate::cardtext::CardTexts>()
            .init_resource::<crate::face::FaceMode>()
            .init_resource::<crate::sheen::Sheen>()
            .init_resource::<crate::touch::Touched>()
            .init_resource::<ledge::LedgeRevision>()
            .init_resource::<ledge::LedgeLayout>()
            .init_resource::<ledge::drawer::DrawerRevision>()
            .init_resource::<ledge::pool::PoolRevision>()
            .init_resource::<ledge::menu::MenuRevision>()
            .init_resource::<tray::TrayRevision>()
            .init_resource::<tray::TrayReveal>()
            .init_resource::<super::SheetRevision>()
            .insert_resource(super::UiSheets {
                parchment: Handle::default(),
            })
            // All of them, chained, in the order the app runs them: the
            // first spawns the shelf, the three retained attachments and the
            // drawer's node, the second writes the shelf and records where
            // its middle ended up, then the drawer is filled over that middle
            // and opened or shut, the pool's row is reconciled and moved, and
            // the game menu's panel is filled and moved. A harness that ran
            // only the rebuild would be reading a bar with no words on it and
            // calling that an answer.
            //
            // The zone dialog is the last and hangs off the same root on a
            // gate of its own, which is a thing a harness running only
            // `sync_overlay` could no longer see at all.
            .add_systems(
                Update,
                (
                    sync_overlay,
                    ledge::sync_ledge,
                    ledge::drawer::sync_drawer,
                    ledge::drawer::zoom_the_drawer,
                    ledge::pool::sync_pool,
                    ledge::pool::zoom_the_pool,
                    ledge::pool::grow_the_pool,
                    ledge::menu::sync_menu,
                    ledge::menu::grow_the_menu,
                    tray::sync_tray,
                    tray::reveal_tray,
                    // The parchment leaf, which since AX 6c draws *both*
                    // choosers — a permanent's abilities and the ways a card
                    // in hand can be cast. A harness that ran the drawer and
                    // not this one could watch a row leave the drawer and
                    // would have nothing to say about where it went.
                    super::sync_ability_sheet,
                )
                    .chain(),
            );
        app.update();
        app
    }

    /// Every word the bar put on the screen.
    ///
    /// `TextSpan` and not `Text`: `slip_text` splits a line into runs so a
    /// bracketed aside can be greyed, which leaves the `Text` itself empty
    /// and every word in a child.
    #[test]
    fn combat_commit_buttons_update_without_a_new_game_snapshot() {
        use baylee_core::ids::Defender;
        use baylee_engine::choice::Pending;
        for blocking in [false, true] {
            let creature = ObjectId::new(3, 0);
            let attacker = ObjectId::new(4, 0);
            let pending = if blocking {
                Pending::ChooseBlockers {
                    player: PlayerId::new(0),
                    attacker: PlayerId::new(1),
                    blockers: vec![baylee_engine::choice::BlockOption {
                        blocker: creature,
                        attackers: vec![attacker],
                    }],
                }
            } else {
                Pending::ChooseAttackers {
                    player: PlayerId::new(0),
                    attackers: vec![creature],
                    defenders: vec![Defender::Player(PlayerId::new(1))],
                }
            };
            let mut duel = duel_with(false);
            duel.receive_choice(pending);
            let mut app = bar_of(duel);
            let label = if blocking {
                Phrase::Block
            } else {
                Phrase::Attack
            }
            .text(Lang::En);
            assert!(!said(&mut app).iter().any(|s| s == label));
            {
                let mut duel = app.world_mut().resource_mut::<Duel>();
                let i = duel.interaction.as_mut().unwrap();
                if blocking {
                    assert!(i.declare_blocker(creature, attacker));
                } else {
                    assert!(i.declare_attacker(creature, Defender::Player(PlayerId::new(1))));
                }
            }
            app.update();
            assert!(said(&mut app).iter().any(|s| s == label), "missing {label}");
        }
    }

    #[test]
    fn tray_image_arrivals_preserve_entities_and_scroll_and_relayout_keeps_offset() {
        let mut duel = duel_with(false);
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        duel.browser.open();
        let mut app = bar_of(duel);
        let list = app
            .world_mut()
            .query_filtered::<Entity, With<tray::TrayScroll>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .entity_mut(list)
            .get_mut::<ScrollPosition>()
            .unwrap()
            .y = 160.0;
        app.world_mut().resource_mut::<CardTextures>().mark_arrived(
            baylee_client_core::images::ImageKey::new(
                baylee_core::ids::PrintRef::new(55),
                0,
                baylee_client_core::images::ArtSize::Small,
            ),
        );
        app.update();
        assert!((app.world().get::<ScrollPosition>(list).unwrap().y - 160.0).abs() < f32::EPSILON);
        app.world_mut()
            .resource_mut::<tray::TrayRevision>()
            .relayout();
        app.update();
        let scroll = app
            .world_mut()
            .query_filtered::<&ScrollPosition, With<tray::TrayScroll>>()
            .single(app.world())
            .unwrap();
        assert!((scroll.y - 160.0).abs() < f32::EPSILON);
    }

    fn said(app: &mut App) -> Vec<String> {
        let mut roots = app.world_mut().query::<&Text>();
        let mut lines: Vec<String> = roots.iter(app.world()).map(|t| t.0.clone()).collect();
        let mut spans = app.world_mut().query::<&TextSpan>();
        lines.extend(spans.iter(app.world()).map(|s| s.0.clone()));
        lines
    }

    /// #182/#140: selecting only a player must rebuild the confirmation row.
    #[test]
    fn selecting_a_player_refreshes_the_confirm_button() {
        use baylee_engine::choice::{Pending, TargetPrompt};
        let mut duel = duel_saying(false, false);
        duel.last_error = None;
        duel.interaction = Some(baylee_client_core::Interaction::new(
            Pending::ChooseTargets {
                player: PlayerId::new(0),
                options: vec![],
                player_options: vec![PlayerId::new(0), PlayerId::new(1)],
                min: 1,
                max: 1,
                reason: TargetPrompt::Targets,
            },
            PlayerId::new(0),
        ));
        let mut app = bar_of(duel);
        let confirms = |app: &mut App| {
            app.world_mut()
                .query::<&PromptButton>()
                .iter(app.world())
                .filter(|button| button.action == PromptAction::Confirm)
                .count()
        };
        assert_eq!(confirms(&mut app), 0);
        app.world_mut()
            .resource_mut::<Duel>()
            .interaction
            .as_mut()
            .unwrap()
            .toggle_player(PlayerId::new(1));
        app.update();
        assert_eq!(confirms(&mut app), 1);
        app.world_mut()
            .resource_mut::<Duel>()
            .interaction
            .as_mut()
            .unwrap()
            .toggle_player(PlayerId::new(1));
        app.update();
        assert_eq!(confirms(&mut app), 0);
    }

    /// The refusal the engine handed back on the last action anyone took.
    const REFUSED: &str = "illegal action for your seat";

    fn duel_with(over: bool) -> Duel {
        duel_saying(over, true)
    }

    /// The same, with the word about the connection left out.
    ///
    /// The shelf shows **one** sentence (AX §6), so a refusal and a lost
    /// socket cannot both be read at once — the socket wins, because a
    /// question answered into a table that is not there arrives nowhere. On
    /// the slip they were two stacked lines and both were drawn.
    fn duel_saying(over: bool, unreachable: bool) -> Duel {
        let pending = if over {
            baylee_engine::choice::Pending::GameOver(GameResult {
                winner: Some(Victor::Player(PlayerId::new(0))),
                reason: EndReason::LastPlayerStanding,
            })
        } else {
            baylee_engine::choice::Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(baylee_engine::choice::LegalActions::default()),
            }
        };
        let mut duel = Duel {
            interaction: Some(baylee_client_core::Interaction::new(
                pending,
                PlayerId::new(0),
            )),
            last_error: Some(baylee_client_core::i18n::Refusal::Verbatim(
                REFUSED.to_string(),
            )),
            link_note: unreachable.then_some(Phrase::LinkLost),
            ..Duel::default()
        };
        // The bar is not drawn at all without a board to draw it over, which
        // is what an empty tree would otherwise be mistaken for.
        duel.view = Some(baylee_client_core::test_support::ViewBuilder::new(2).build());
        crate::rebuild_board(&mut duel);
        duel
    }

    /// A priority with something on the stack to let go of, and no hold
    /// running.
    /// A seat with one spell on the stack, and a table for it to be on.
    ///
    /// The roster is not decoration and was missing: `sync_overlay` draws the
    /// stack panel only for a duel whose `statics` have arrived, so this
    /// helper promised a stack in its name and drew the shelf's hold button
    /// and nothing else. Nothing was asserting vacuously because of it — the
    /// two tests on it claim about the shelf — but the next negative claim
    /// about the panel would have been, which is the shape that counts a
    /// green test that checks nothing.
    ///
    /// `statics` gates the **hand zone** as well, and no other `duel_*`
    /// builder here carries one either. Those are sound today for the same
    /// reason and for no better one.
    fn duel_with_a_stack() -> Duel {
        let mut duel = duel_saying(false, false);
        let view = duel.view.as_mut().expect("the seat has a view");
        view.stack = vec![baylee_client_core::test_support::token(9, 1, "Shock", 0, 0)];
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        crate::rebuild_board(&mut duel);
        duel
    }

    /// A seat that is not being asked, because it said not to ask.
    ///
    /// The question belongs to the *other* player, which is what makes this
    /// seat wait: `Interaction::is_mine` compares the pending's player with
    /// the seat, and everything the shelf calls "waiting" comes off that one
    /// answer.
    fn duel_not_asking(hold: bool, pilot: bool) -> Duel {
        let mut duel = Duel {
            interaction: Some(baylee_client_core::Interaction::new(
                baylee_engine::choice::Pending::Priority {
                    player: PlayerId::new(1),
                    legal: Box::new(baylee_engine::choice::LegalActions::default()),
                },
                PlayerId::new(0),
            )),
            autopilot: pilot
                .then_some(baylee_client_core::automation::AutoPilot::ToNextTurn { from_turn: 1 }),
            ..Duel::default()
        };
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        view.priority_held = hold;
        duel.view = Some(view);
        crate::rebuild_board(&mut duel);
        duel
    }

    /// A seat being asked to name a colour, which is the shortest question
    /// that needs a drawer: the answer is a list, and a list is more than a
    /// line.
    fn duel_choosing_a_colour() -> Duel {
        use baylee_core::mana::ManaColor;
        let mut duel = Duel {
            interaction: Some(baylee_client_core::Interaction::new(
                baylee_engine::choice::Pending::ChooseColor {
                    player: PlayerId::new(0),
                    options: vec![ManaColor::White, ManaColor::Blue, ManaColor::Black],
                },
                PlayerId::new(0),
            )),
            ..Duel::default()
        };
        duel.view = Some(baylee_client_core::test_support::ViewBuilder::new(2).build());
        crate::rebuild_board(&mut duel);
        duel
    }

    /// The drawer opens on a question whose answer is a list, and is shut the
    /// rest of the time.
    ///
    /// Both halves, because each is a different bug. A drawer that never
    /// opens is a colour chooser drawn as "Choose a colour" with nothing
    /// under it, which is how a tapped dual land used to stop a game dead. A
    /// drawer that never shuts is an empty panel standing over the table for
    /// the whole of every turn — and its node is exempt from the overlay's
    /// sweep now, so nothing else would take it away.
    ///
    /// It counts [`ChoiceButton`]s rather than reading words: a colour is
    /// answered by a mana pip and carries no label at all, on the grounds
    /// that a `{U}` disc says "blue" in every language there is.
    #[test]
    fn the_drawer_opens_on_a_list_and_is_shut_otherwise() {
        let rows = |app: &mut App| {
            let mut q = app.world_mut().query::<&ChoiceButton>();
            q.iter(app.world()).map(|b| b.index).collect::<Vec<_>>()
        };
        let panels = |app: &mut App| {
            let mut q = app
                .world_mut()
                .query_filtered::<Option<&Children>, With<ledge::drawer::DrawerRoot>>();
            q.iter(app.world())
                .map(|c| c.map_or(0, bevy::ecs::hierarchy::Children::len))
                .sum::<usize>()
        };

        let mut app = bar_of(duel_choosing_a_colour());
        assert_eq!(
            rows(&mut app),
            vec![0, 1, 2],
            "three colours were offered and the drawer has to carry all three"
        );
        assert_eq!(panels(&mut app), 1, "and they stand on one panel");

        // The same seat, with nothing being asked of it.
        let mut app = bar_of(duel_with(false));
        assert!(
            rows(&mut app).is_empty(),
            "a priority window is answered on the shelf, not out of a drawer"
        );
        assert_eq!(
            panels(&mut app),
            0,
            "an empty drawer is a panel standing over the table saying nothing"
        );
    }

    /// The panel is kept while the question is, and it leaves with it.
    ///
    /// §7 gives the drawer the one movement on this shelf that overshoots:
    /// it *appears*, where an answer merely *changes*. That only reads right
    /// if "appears" happens once — a panel rebuilt whenever its contents
    /// changed would pop again on the row a player has just taken, which is a
    /// movement saying "something new arrived" about the click they just
    /// made. So the identity of the entity is the assertion: the same panel
    /// across a change of contents, and **no** panel once the question is
    /// gone. The second half is what
    /// [`the_drawer_opens_on_a_list_and_is_shut_otherwise`] cannot reach —
    /// it builds a fresh app per case, so nothing there has ever had to
    /// leave.
    #[test]
    fn the_drawers_panel_outlives_its_contents_and_not_its_question() {
        let panel = |app: &mut App| {
            let mut q = app
                .world_mut()
                .query_filtered::<&Children, With<ledge::drawer::DrawerRoot>>();
            q.iter(app.world()).flatten().copied().next()
        };

        let mut app = bar_of(duel_choosing_a_colour());
        let opened = panel(&mut app).expect("a colour is chosen out of the drawer");

        // The middle colour is taken. The rows are redrawn — one of them is
        // washed now — and the drawer has not opened again.
        {
            let mut duel = app.world_mut().resource_mut::<Duel>();
            assert!(
                duel.interaction.as_mut().is_some_and(|i| i.choose_index(1)),
                "the second colour is one of the three that were offered"
            );
        }
        app.update();
        assert_eq!(
            panel(&mut app),
            Some(opened),
            "a row being taken is a change of contents, not a second arrival"
        );

        // And the question goes away. With motion off the way out is over on
        // the frame it starts, so the panel is gone by the end of this one.
        *app.world_mut().resource_mut::<Duel>() = duel_with(false);
        app.update();
        assert_eq!(
            panel(&mut app),
            None,
            "the drawer has to be able to leave, not merely to stop being filled"
        );
    }

    /// Once a result is standing, this overlay offers nothing.
    ///
    /// Four things, and only the first of them was ever silenced. A refusal
    /// or a word about the connection each draws the whole slip on its own,
    /// under the end screen and in the veil, saying something about a game
    /// that has stopped being played — and the draw and concede controls
    /// stayed lit, hovering under the pointer and answering nothing, because
    /// `DuelSet::Input` does not run in `Finished`.
    ///
    /// Those two are drawn by `ledge::ways_out` now rather than in the corner
    /// up here, and the test did not move with them: `said` reads the whole
    /// world, so it is about what the player can see and never about which
    /// function put it there.
    #[test]
    fn nothing_the_overlay_offers_outlives_the_game() {
        let mut app = bar_of(duel_with(true));
        let lines = said(&mut app);
        assert!(
            !lines.iter().any(|l| l.contains(REFUSED)),
            "a refusal outlived the game it refused: {lines:?}"
        );
        let note = Phrase::LinkLost.text(Lang::En).to_string();
        assert!(
            !lines.iter().any(|l| l.contains(&note)),
            "the table is gone and so is the reason to say so: {lines:?}"
        );
        for pill in [Phrase::OfferADraw, Phrase::Concede] {
            let label = pill.text(Lang::En).to_string();
            assert!(
                !lines.contains(&label),
                "a way to end a game that has ended, lit and unanswerable: \
                 {lines:?}"
            );
        }
    }

    /// The counter-test, without which the one above passes on an empty tree.
    ///
    /// It is not a formality here: the first draft of this harness built no
    /// tree at all — `sync_overlay` returns early without a board — and the
    /// test above passed on the empty world.
    #[test]
    fn a_game_still_being_played_is_offered_all_four() {
        let mut app = bar_of(duel_with(false));
        let lines = said(&mut app);
        let note = Phrase::LinkLost.text(Lang::En).to_string();
        assert!(
            lines.iter().any(|l| l.contains(&note)),
            "a table that cannot hear you has to say so: {lines:?}"
        );
        // The refusal is there too, and it is the only sentence the shelf
        // draws once the socket is back: one line, and the more urgent of the
        // two takes it.
        let mut reachable = bar_of(duel_saying(false, false));
        let lines = said(&mut reachable);
        assert!(
            lines.iter().any(|l| l.contains(REFUSED)),
            "an answer the engine turned down has to be readable: {lines:?}"
        );
        // The two ways out are behind the burger now, so the shelf carries
        // the mark and not the words. Both halves are asserted: a door that
        // is drawn, and nothing of what is behind it spelled out beside it.
        assert!(
            lines.contains(&glyph::BARS.to_string()),
            "a game still being played has a way out to reach for: {lines:?}"
        );
        for pill in [Phrase::OfferADraw, Phrase::Concede] {
            let label = pill.text(Lang::En).to_string();
            assert!(
                !lines.contains(&label),
                "a shut menu says nothing of what is in it: {lines:?}"
            );
        }
        // And opened, it says both. Through the same `MenuAction` a click
        // sends, rather than by writing the flag, so the test cannot pass on
        // a panel no button can reach — [[client-tests-must-answer-like-a-
        // player]], one level down from a `PlayerAction`.
        crate::input::menu_click(
            &mut reachable.world_mut().resource_mut::<Duel>(),
            MenuAction::ToggleGameMenu,
            false,
        );
        reachable.update();
        let lines = said(&mut reachable);
        for pill in [Phrase::OfferADraw, Phrase::Concede] {
            let label = pill.text(Lang::En).to_string();
            assert!(
                lines.contains(&label),
                "every game still being played offers both of these: {lines:?}"
            );
        }
        assert!(
            lines.iter().any(|l| l.contains(baylee_build::short())),
            "and says which baylee it is: {lines:?}"
        );
    }

    /// A refusal this client wrote is read in the player's language, and one
    /// another process sent is read in its own.
    ///
    /// The joint, and it had no test: `Phrase` was checked for having a
    /// German arm (`i18n`'s two suite-wide tests) and the shelf was checked
    /// for drawing *a* refusal (`a_game_still_being_played_is_offered_all_
    /// four`), and the line between them — that the sentence reaching the
    /// player is the translated one — was asserted by nothing. That is the
    /// "declared but never wired" shape this crate keeps finding, and here
    /// it had shipped: `Duel::last_error` was a `String`, so nine sentences
    /// this client writes were English on a German screen (#121).
    ///
    /// Both arms in one test on purpose. Asserting only the German half
    /// would pass on a client that translated *everything* in that slot,
    /// which is the opposite defect and the one that would silently rewrite
    /// an engine's refusal into a sentence this client made up.
    #[test]
    fn a_refusal_is_read_in_the_language_of_whoever_wrote_it() {
        // `mine` and not `said`, which is the function three lines down that
        // reads the screen.
        for (lang, mine) in [
            (Lang::De, Phrase::DeedWithdrawn.text(Lang::De)),
            (Lang::En, Phrase::DeedWithdrawn.text(Lang::En)),
        ] {
            // `duel_saying(_, false)` and never `duel_with`: a lost socket
            // takes the one sentence the shelf draws (AX §6), and this test
            // is about what is written in it.
            let mut duel = duel_saying(false, false);
            duel.last_error = Some(baylee_client_core::i18n::Refusal::Said(
                Phrase::DeedWithdrawn,
            ));
            let mut app = bar_of(duel);
            app.world_mut()
                .resource_mut::<crate::settings::ClientSettings>()
                .lang = lang.code().to_string();
            app.update();
            let lines = said(&mut app);
            assert!(
                lines.iter().any(|l| l.contains(mine)),
                "a sentence this client wrote is read in {lang:?}: {lines:?}"
            );
            let other = Phrase::DeedWithdrawn.text(match lang {
                Lang::De => Lang::En,
                Lang::En => Lang::De,
            });
            assert!(
                !lines.iter().any(|l| l.contains(other)),
                "and only in {lang:?} — the other language is on the shelf \
                 too: {lines:?}"
            );
        }

        // The other arm, unchanged and staying that way: prose another
        // process sent is drawn as it came, in a German interface, because
        // the process that said no is the one that knows why.
        let mut app = bar_of(duel_saying(false, false));
        app.world_mut()
            .resource_mut::<crate::settings::ClientSettings>()
            .lang = Lang::De.code().to_string();
        app.update();
        let lines = said(&mut app);
        assert!(
            lines.iter().any(|l| l.contains(REFUSED)),
            "another process's refusal is not this client's to translate or \
             to drop: {lines:?}"
        );
    }

    /// The panel, its entity and whether it is on the screen.
    fn menu_panel(app: &mut App) -> Option<(Entity, bool, usize)> {
        let mut q = app
            .world_mut()
            .query_filtered::<(Entity, &Visibility, Option<&Children>), With<ledge::menu::MenuPanel>>(
            );
        q.iter(app.world()).next().map(|(e, seen, kids)| {
            (
                e,
                *seen != Visibility::Hidden,
                kids.map_or(0, bevy::ecs::hierarchy::Children::len),
            )
        })
    }

    /// The scale the panel is drawn at, and where that scale is anchored.
    fn menu_pose(app: &mut App) -> (Vec2, Val2) {
        let mut q = app
            .world_mut()
            .query_filtered::<&UiTransform, With<ledge::menu::MenuPanel>>();
        let at = q.single(app.world()).expect("one panel");
        (at.scale, at.translation)
    }

    /// **The whole reason the panel is not a child of the shelf.**
    ///
    /// Arming the concession writes `LedgeRevision::concede_armed`, which
    /// despawns and rebuilds the shelf's columns. The second press has to be
    /// made in the panel, so the panel has to be the *same entity* on the
    /// other side of that rebuild — and it has to still be up, with its rows
    /// redrawn to the confirm wording rather than left saying what they said
    /// before.
    ///
    /// Three claims and the first is the one a design that got this wrong
    /// would fail: same entity, still shown, and the confirm wording drawn.
    #[test]
    fn the_panel_survives_the_rebuild_the_arming_press_causes() {
        let mut app = bar_of(duel_with(false));
        crate::input::menu_click(
            &mut app.world_mut().resource_mut::<Duel>(),
            MenuAction::ToggleGameMenu,
            false,
        );
        app.update();
        let (panel, shown, rows) = menu_panel(&mut app).expect("a panel");
        assert!(shown, "the menu is open");
        assert_eq!(rows, 4, "two ways out, a rule and the version");
        // The shelf's own children, less the two casts: those are spawned
        // with the shelf and exempt from its rebuild, so counting them would
        // make "everything was rebuilt" false on a shelf that rebuilt
        // everything it rebuilds.
        let columns = |app: &mut App| {
            let casts = {
                let mut q = app
                    .world_mut()
                    .query_filtered::<Entity, With<ledge::LedgeCast>>();
                q.iter(app.world()).collect::<Vec<_>>()
            };
            let mut q = app
                .world_mut()
                .query_filtered::<&Children, With<ledge::LedgeShelf>>();
            q.iter(app.world())
                .flat_map(|c| c.iter().collect::<Vec<_>>())
                .filter(|e| !casts.contains(e))
                .collect::<Vec<_>>()
        };
        let before = columns(&mut app);

        // The arming press, through the same door a click uses.
        crate::input::menu_click(
            &mut app.world_mut().resource_mut::<Duel>(),
            MenuAction::Concede,
            false,
        );
        assert!(app.world().resource::<Duel>().concede_armed, "armed");
        app.update();

        // The premise, proved rather than assumed: this test says nothing at
        // all if the shelf did not in fact rebuild, and a shelf that stopped
        // rebuilding would let every claim below pass for the wrong reason.
        let after = columns(&mut app);
        assert!(
            after.iter().all(|e| !before.contains(e)),
            "the arming press did not rebuild the shelf, so this test is not \
             about anything: {before:?} then {after:?}"
        );

        let (again, still, rows_now) = menu_panel(&mut app).expect("still a panel");
        assert_eq!(
            panel, again,
            "the panel the second press has to be made in was despawned by the \
             first press"
        );
        assert!(still, "and it is still on the screen");
        assert_eq!(
            rows_now, rows,
            "and nothing left the column, so the confirm row did not move up \
             under the pointer that is about to press it"
        );
        let lines = said(&mut app);
        let confirm = Phrase::ConcedeConfirm.text(Lang::En).to_string();
        assert!(
            lines.contains(&confirm),
            "and it now asks for the second press: {lines:?}"
        );
        // The draw offer keeps its place and loses its handle, which is what
        // `Weight::Dead` is: drawn, and not a control.
        let offered = {
            let mut q = app.world_mut().query::<&MenuButton>();
            q.iter(app.world())
                .filter(|b| b.action == MenuAction::OfferDraw)
                .count()
        };
        assert_eq!(
            offered, 0,
            "a draw cannot be offered in the middle of conceding, and a row \
             that answers nothing must not be pressable"
        );
    }

    /// The panel grows out of the shelf's right corner rather than appearing.
    ///
    /// Three claims, and the corner is the one a plain [`motion::from_bottom`]
    /// would break: on the frame the menu opens the panel is drawn at
    /// [`motion::ZOOM_FROM`], pinned at its **bottom-right** — because a node
    /// fixed at the right margin that shrinks toward its own middle slides
    /// left as it grows, away from the button that opened it. Then the
    /// movement ends at full size.
    ///
    /// With `reduce_motion` it is at full size on that same first frame,
    /// which is the counter-test for the first claim on its own terms.
    #[test]
    fn the_panel_grows_out_of_the_corner_its_button_is_in() {
        for (still, want) in [(false, motion::ZOOM_FROM), (true, 1.0)] {
            let mut app = bar_of(duel_with(false));
            app.world_mut()
                .resource_mut::<crate::prefs::Prefs>()
                .edit()
                .reduce_motion = still;
            assert!(
                !menu_panel(&mut app).expect("a panel").1,
                "a menu nobody opened is not on the screen"
            );

            crate::input::menu_click(
                &mut app.world_mut().resource_mut::<Duel>(),
                MenuAction::ToggleGameMenu,
                false,
            );
            app.update();

            assert!(menu_panel(&mut app).expect("a panel").1, "and now it is");
            let (scale, shift) = menu_pose(&mut app);
            assert!(
                (scale.x - want).abs() < 0.001 && (scale.y - want).abs() < 0.001,
                "reduce_motion {still}: drawn at {scale:?}, {want} wanted"
            );
            if !still {
                assert_eq!(
                    shift,
                    motion::from_bottom_right(want),
                    "the panel grows out of the corner its button is in"
                );
                tick(&mut app, motion::ZOOM_IN + 0.01);
                let (scale, shift) = menu_pose(&mut app);
                assert!(
                    (scale.x - 1.0).abs() < 0.001,
                    "and the arrival ends at full size, not at {scale:?}"
                );
                assert_eq!(shift, motion::from_bottom_right(1.0));
            }
        }
    }

    /// And it folds back into the shelf rather than being taken off it.
    ///
    /// The clock is driven for the reason
    /// `the_strip_folds_back_into_the_shelf_rather_than_being_taken_off_it`
    /// records: `bar_of` has no running time, so a test written against the
    /// still clock would read the frame the fold *starts* as the frame it
    /// ends and would pass against a `grow_the_menu` that hid the panel
    /// outright.
    #[test]
    fn the_panel_folds_away_rather_than_being_taken_away() {
        let mut app = bar_of(duel_with(false));
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = false;
        crate::input::menu_click(
            &mut app.world_mut().resource_mut::<Duel>(),
            MenuAction::ToggleGameMenu,
            false,
        );
        app.update();
        assert!(menu_panel(&mut app).expect("a panel").1, "open");

        crate::input::menu_click(
            &mut app.world_mut().resource_mut::<Duel>(),
            MenuAction::ToggleGameMenu,
            false,
        );
        tick(&mut app, 0.0);
        assert!(
            menu_panel(&mut app).expect("a panel").1,
            "the panel folds away rather than being taken away"
        );
        let (scale, _) = menu_pose(&mut app);
        assert!(
            (scale.x - 1.0).abs() < 0.001,
            "and the fold begins at full size, not at {scale:?}"
        );

        tick(&mut app, motion::ZOOM_OUT + 0.01);
        assert!(
            !menu_panel(&mut app).expect("a panel").1,
            "and once the fold is over it is away"
        );
    }

    /// And a game that has ended offers neither, even asked directly.
    ///
    /// `nothing_the_overlay_offers_outlives_the_game`'s other half now that
    /// the pair is behind a door: that test reads what is *drawn*, and a shut
    /// menu draws nothing either way, so on its own it would pass over a
    /// panel that still filled itself with two unanswerable buttons. This one
    /// opens it. `DuelSet::Input` does not run in `Finished`, so anything
    /// drawn in there would warm under the pointer and answer nothing —
    /// exactly what the pair used to do in the corner.
    #[test]
    fn the_menu_offers_no_way_out_of_a_game_that_has_ended() {
        let mut app = bar_of(duel_with(true));
        app.world_mut().resource_mut::<Duel>().game_menu = true;
        app.update();
        let lines = said(&mut app);
        for pill in [Phrase::OfferADraw, Phrase::Concede, Phrase::ConcedeConfirm] {
            let label = pill.text(Lang::En).to_string();
            assert!(
                !lines.contains(&label),
                "a way to end a game that has ended, opened and unanswerable: \
                 {lines:?}"
            );
        }
        assert!(
            !lines.contains(&glyph::BARS.to_string()),
            "and the door itself is gone with the column it stood in: {lines:?}"
        );
    }

    /// A running hold says so in the middle, where the question would be.
    ///
    /// It is the one game state with **no other symptom** (AX §4.4): the
    /// middle is empty precisely *because* the seat is not being asked, which
    /// is what an idle shelf looks like — so a player who set a hold two
    /// turns ago and forgot would watch the game play itself with nothing on
    /// screen to blame.
    ///
    /// Two mechanisms and one picture, and this is where that is asserted
    /// rather than merely intended: an engine hold and the client's own
    /// autopilot each put the same sentence and the same way out on the
    /// shelf. Live, neither is a state a screenshot can be relied on to
    /// catch — the house AI answers a whole turn between two frames.
    #[test]
    fn a_running_hold_is_drawn_where_the_question_would_have_been() {
        for (hold, pilot, what) in [
            (true, false, "an engine hold"),
            (false, true, "the autopilot"),
        ] {
            let mut app = bar_of(duel_not_asking(hold, pilot));
            let lines = said(&mut app);
            for phrase in [Phrase::HoldingPriority, Phrase::HoldRelease] {
                let words = phrase.text(Lang::En).to_string();
                assert!(
                    lines.contains(&words),
                    "{what} left the shelf with nothing to blame: {lines:?}"
                );
            }
        }
        // The counter-half, and the reason the sentence is not simply always
        // there: a seat that *is* being asked has a question of its own, and
        // two sentences on one shelf is what §6 forbids.
        let mut app = bar_of(duel_not_asking(false, false));
        let lines = said(&mut app);
        let words = Phrase::HoldingPriority.text(Lang::En).to_string();
        assert!(
            !lines.contains(&words),
            "nobody is holding anything and the shelf said otherwise: {lines:?}"
        );
    }

    /// Everything the root carries **except** the nodes the sweep is told to
    /// pass over.
    ///
    /// Naming the entities is the honest way to say "what the rebuild
    /// rebuilds": it means the nodes it lists rather than resting on a
    /// component that happens to be drawn in one place — `MenuButton` was
    /// that component and stopped being it the moment the ways out of a game
    /// moved to the shelf.
    ///
    /// Five now, not two: the zone dialog's veil and panel joined the list
    /// when the dialog got a revision of its own, and the tray strip when the
    /// dialog gained somewhere to be put down.
    ///
    /// This list and `sync_overlay`'s are hand-kept and separate, which is a
    /// drift waiting to happen — and it did, on the commit that added the
    /// strip. Catching it is the point; that the failure arrives *here* and
    /// not at the sweep is why the caller's assertion has to say both things
    /// it can mean.
    fn nodes_the_rebuild_rebuilds(app: &mut App) -> Vec<Entity> {
        let kept = {
            let mut q = app.world_mut().query_filtered::<Entity, Or<(
                With<ledge::LedgeShelf>,
                With<ledge::drawer::DrawerRoot>,
                With<TableVeil>,
                With<TrayBand>,
                With<ledge::tray::TrayStrip>,
                With<ledge::pool::PoolStrip>,
                With<ledge::menu::MenuPanel>,
            )>>();
            q.iter(app.world()).collect::<Vec<_>>()
        };
        let mut q = app.world_mut().query_filtered::<&Children, With<HudRoot>>();
        q.iter(app.world())
            .flat_map(|c| c.iter().collect::<Vec<_>>())
            .filter(|e| !kept.contains(e))
            .collect::<Vec<_>>()
    }

    /// The shelf is built once and stands; everything else on the overlay is
    /// a picture of the snapshot and is drawn again.
    ///
    /// [`HudRevision`] counts the hover, so this rebuild happens hundreds of
    /// times a turn — every time the pointer crosses a card. The shelf is the
    /// one node that must survive it: a `Feel`'s warmth is state on the
    /// button entity, so a shelf torn down under the pointer snaps the button
    /// the player is reaching for back to rest, and the ledge's own revision
    /// counter would govern a subtree that is deleted before it can be
    /// compared.
    ///
    /// The counter-half of the assertion is the important one. Without it
    /// this passes on an overlay that stopped rebuilding at all, which is the
    /// much worse bug of the two: a bar that never redraws says the wrong
    /// thing about the game for as long as the game lasts.
    #[test]
    #[allow(clippy::too_many_lines)] // five queries, each read twice
    fn the_shelf_and_its_attachments_outlive_a_rebuild_and_nothing_else_does() {
        // A print table, because the counter-half needs something the overlay
        // *does* draw under the root, and everything it draws there asks for
        // one. The two ways out of a game were the exception — spawned with
        // nothing but a language — and they are on the shelf now (AX §4.3),
        // so what answers for the overlay is the hand zone.
        let mut duel = duel_with(false);
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        let mut app = bar_of(duel);

        let shelf = |app: &mut App| {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<ledge::LedgeShelf>>();
            q.iter(app.world()).collect::<Vec<_>>()
        };
        let roots = |app: &mut App| {
            let mut q = app.world_mut().query_filtered::<Entity, With<HudRoot>>();
            q.iter(app.world()).collect::<Vec<_>>()
        };
        // What the shelf carries, and what the overlay carries: the two sides
        // of the claim.
        //
        // The shelf's two casts are not columns and are filtered out: they
        // are spawned with the shelf and exempt from the rebuild, so counting
        // them here would make "three columns" read five and would let a
        // column that stopped being built pass unnoticed.
        let standing = |app: &mut App| {
            let casts = {
                let mut q = app
                    .world_mut()
                    .query_filtered::<Entity, With<ledge::LedgeCast>>();
                q.iter(app.world()).collect::<Vec<_>>()
            };
            let mut q = app
                .world_mut()
                .query_filtered::<&Children, With<ledge::LedgeShelf>>();
            q.iter(app.world())
                .flat_map(|c| c.iter().collect::<Vec<_>>())
                .filter(|e| !casts.contains(e))
                .collect::<Vec<_>>()
        };
        let drawer = |app: &mut App| {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<ledge::drawer::DrawerRoot>>();
            q.iter(app.world()).collect::<Vec<_>>()
        };
        // The three strips beside them, each surviving for an argument of
        // its own — `OverlayTree`'s own fields say which. They are read as
        // one list because the *claim* is identical for all three and the
        // sweep is one condition per strip, so a fourth attachment costs one
        // line here rather than a paragraph, and the message still names
        // which of them went.
        //
        // They are checked by **identity** because the sweep does not leave a
        // hole behind it: a strip despawned here is spawned again on the same
        // frame, by the branch below that builds the root. So the count is
        // one either way, the picture is right on the next frame, and what is
        // actually lost is the `MenuZoom` or the `PoolReveal` that was on the
        // old entity — which is why removing any one of those three
        // conditions from the sweep passed every test in this file.
        let strips = |app: &mut App| {
            let mut tray = app
                .world_mut()
                .query_filtered::<Entity, With<ledge::tray::TrayStrip>>();
            let tray = tray.iter(app.world()).collect::<Vec<_>>();
            let mut pool = app
                .world_mut()
                .query_filtered::<Entity, With<ledge::pool::PoolStrip>>();
            let pool = pool.iter(app.world()).collect::<Vec<_>>();
            let mut menu = app
                .world_mut()
                .query_filtered::<Entity, With<ledge::menu::MenuPanel>>();
            let menu = menu.iter(app.world()).collect::<Vec<_>>();
            [("tray", tray), ("mana pool", pool), ("game menu", menu)]
        };

        let was_shelf = shelf(&mut app);
        let was_drawer = drawer(&mut app);
        let was_root = roots(&mut app);
        let was_standing = standing(&mut app);
        let was_strips = strips(&mut app);
        let was_redrawn = nodes_the_rebuild_rebuilds(&mut app);
        assert_eq!(was_shelf.len(), 1, "one shelf, and it was built");
        assert_eq!(was_drawer.len(), 1, "and one drawer beside it");
        assert_eq!(was_root.len(), 1, "and one root to hang them off");
        assert_eq!(
            was_standing.len(),
            3,
            "hand tools, answers, and game controls — the mana pool left this \
             node for a strip of its own"
        );
        assert!(
            !was_redrawn.is_empty(),
            "the overlay drew something of its own beside the two"
        );
        for (name, found) in &was_strips {
            assert_eq!(found.len(), 1, "one {name}, and it was built");
        }

        // The pointer moves onto a card. Nothing about the game changed.
        app.world_mut().resource_mut::<Duel>().hovered = Some(ObjectId::new(1, 0));
        app.update();

        assert_eq!(shelf(&mut app), was_shelf, "the shelf was rebuilt");
        assert_eq!(
            drawer(&mut app),
            was_drawer,
            "the drawer's node was rebuilt, and a panel open under the \
             pointer would have gone with it"
        );
        assert_eq!(roots(&mut app), was_root, "and so was the root under it");
        let now_strips = strips(&mut app);
        for ((name, was), (_, now)) in was_strips.iter().zip(now_strips.iter()) {
            assert_eq!(
                was, now,
                "the {name} was taken off the root and built again. Nothing                  else can do that — it is spawned once, with the root — so                  the sweep above has stopped sparing it, and whatever the                  strip was in the middle of has been thrown away with the                  entity that was doing it"
            );
        }
        assert_eq!(
            standing(&mut app),
            was_standing,
            "the shelf kept its place and lost what was on it, which is the \
             same loss one level down: a `Feel` under the pointer goes back \
             to rest"
        );
        let now_redrawn = nodes_the_rebuild_rebuilds(&mut app);
        assert!(!now_redrawn.is_empty(), "the overlay still draws it");
        // The count is what separates the two things this can mean, so it is
        // in the message: *all* of them surviving is a rebuild that stopped,
        // and one of them is a new attachment beside the shelf that `kept`
        // above has not been told about.
        let stale = now_redrawn
            .iter()
            .filter(|e| was_redrawn.contains(e))
            .count();
        assert!(
            stale == 0,
            "{stale} of {} nodes under the root came through the rebuild \
             unchanged. All of them means the overlay stopped rebuilding and \
             is showing the tree it built for a different frame; one or two \
             means something new stands beside the shelf and this test's \
             `kept` list has not been told — `sync_overlay`'s own sweep is \
             the list to hold it against.",
            now_redrawn.len()
        );
    }

    /// The owner's report, as an assertion: *„Das Zonen-Dialog ist noch sehr
    /// instabil! Beim Hover flackert alles"*.
    ///
    /// The dialog is a hundred rows and the pointer moves *across* them, so
    /// every row it reached tore the whole overlay down and wrote it again —
    /// the dialog with it, because the dialog was part of that tree. What the
    /// player sees is the row they are reaching for going out: the
    /// replacement is a new entity with a fresh [`Feel`] at `warmth: 0`, and
    /// picking needs a frame to send `Over` to something that did not exist
    /// when it last looked.
    ///
    /// Both halves, and the second is the one that makes the first mean
    /// anything: a dialog that had simply stopped being drawn would pass the
    /// first assertion perfectly. So the filter is typed into next, which is
    /// a change the dialog *must* answer, and the same entity standing there
    /// would be the opposite defect — a panel showing a list nobody narrowed.
    #[test]
    fn the_zone_dialog_outlives_a_pointer_move_and_not_a_search() {
        let mut duel = duel_with(false);
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        duel.browser.open();
        let mut app = bar_of(duel);

        let panel = |app: &mut App| {
            let mut q = app.world_mut().query_filtered::<Entity, With<TrayBand>>();
            q.iter(app.world()).collect::<Vec<_>>()
        };
        let veil = |app: &mut App| {
            let mut q = app.world_mut().query_filtered::<Entity, With<TableVeil>>();
            q.iter(app.world()).collect::<Vec<_>>()
        };
        // The overlay's own half, so this cannot pass on a renderer that has
        // stopped rebuilding anything at all.
        let hand = |app: &mut App| {
            let mut q = app.world_mut().query_filtered::<Entity, With<HandScroll>>();
            q.iter(app.world()).collect::<Vec<_>>()
        };

        let was_panel = panel(&mut app);
        let was_veil = veil(&mut app);
        let was_hand = hand(&mut app);
        assert_eq!(was_panel.len(), 1, "the dialog was drawn at all");
        assert_eq!(was_veil.len(), 1, "and the veil behind it");
        assert_eq!(was_hand.len(), 1, "and the overlay drew its own hand zone");

        // The pointer moves onto a card. Nothing about the dialog changed.
        app.world_mut().resource_mut::<Duel>().hovered = Some(ObjectId::new(1, 0));
        app.update();

        assert_eq!(panel(&mut app), was_panel, "the dialog was rebuilt");
        assert_eq!(veil(&mut app), was_veil, "and so was the veil behind it");
        assert!(
            hand(&mut app).iter().all(|e| !was_hand.contains(e)),
            "the overlay stopped rebuilding, so the dialog standing still \
             says nothing about the dialog"
        );

        // And the counter-half: a letter in the filter box is a different
        // list, and a different list is a rebuild.
        app.world_mut()
            .resource_mut::<Duel>()
            .browser
            .push_filter('a');
        app.update();
        assert!(
            panel(&mut app).iter().all(|e| !was_panel.contains(e)),
            "the search narrowed nothing: the dialog is showing the list it \
             built before the letter was typed"
        );
    }

    /// The owner's other complaint about this dialog: a row grew under the
    /// pointer.
    ///
    /// A row is a **line of writing** — a tick, a picture, a name, the pips,
    /// a type line — and [`Feel::lift`]'s own doc names that as the case that
    /// must stay at zero: growing a row grows the sentence on it, and a
    /// hundred of them under a moving pointer is a list that reflows while it
    /// is being read. `row_slot` carries `Feel::tinting_to` for exactly that
    /// reason, which lights the row and leaves it where it is.
    ///
    /// Both halves, and the second is what makes the first mean anything: a
    /// query that found no rows, or a dialog in which nothing lifts at all,
    /// would pass "every row is at zero" perfectly. So the same question is
    /// put to the **grid**, where a tile is a picture and answers the pointer
    /// by growing — the one place this dialog parts company with itself, and
    /// it is deliberate.
    ///
    /// Both list modes, because `Large` is the detailed row with its asides
    /// dropped and its picture doubled: it is the same `row_slot` and it is
    /// the mode a second hand would forget.
    #[test]
    fn a_row_is_lit_under_the_pointer_and_never_grows() {
        use baylee_client_core::browser::ViewMode;

        fn lifts(app: &mut App) -> Vec<f32> {
            let mut found = app
                .world_mut()
                .query_filtered::<&crate::ambience::Feel, With<TrayCard>>();
            found.iter(app.world()).map(|feel| feel.lift).collect()
        }

        fn dialog_in(mode: ViewMode) -> App {
            // A row and a tile both draw a thumbnail, and asking for one is
            // an `AssetServer::load` — which spawns on the IO pool and panics
            // without it. Idempotent, so several tests may ask.
            bevy::tasks::IoTaskPool::get_or_init(Default::default);
            let mut duel = duel_with(false);
            duel.statics = Some(baylee_client_core::test_support::statics(8));
            duel.view = Some(
                baylee_client_core::test_support::ViewBuilder::new(2)
                    .with_graveyard(
                        0,
                        (10..16)
                            .map(|s| baylee_client_core::test_support::printed(s, 0, "Forest", 1))
                            .collect(),
                    )
                    .build(),
            );
            crate::rebuild_board(&mut duel);
            duel.browser.open();
            let mut app = bar_of(duel);
            // After the harness has run once, so this is the change that
            // rebuilds: the view mode is in `TrayRevision`'s browser gate.
            app.world_mut()
                .resource_mut::<crate::settings::ClientSettings>()
                .zone_view = mode;
            app.update();
            app
        }

        for mode in [ViewMode::Detailed, ViewMode::Large] {
            let mut app = dialog_in(mode);
            let rows = lifts(&mut app);
            assert_eq!(
                rows.len(),
                6,
                "{mode:?}: the list drew a row per card in the pile, and did \
                 not: {rows:?}"
            );
            assert!(
                rows.iter().all(|lift| *lift == 0.0),
                "{mode:?}: a row lifts, so it grows under the pointer and the \
                 sentence on it grows with it: {rows:?}"
            );
        }

        let mut app = dialog_in(ViewMode::Grid);
        let tiles = lifts(&mut app);
        assert_eq!(tiles.len(), 6, "the grid drew a tile per card: {tiles:?}");
        assert!(
            tiles.iter().all(|lift| *lift > 0.0),
            "nothing in this dialog lifts at all, so the rows standing at zero \
             says nothing about the rows: {tiles:?}"
        );
    }

    /// Whether `entity` hangs somewhere under a standing parchment leaf.
    ///
    /// Ancestry and not a count, because AX 6c moved rows between two panels
    /// that draw the **same component**: a cast row is one row of an indexed
    /// choice on the sheet exactly as a colour is one in the drawer, and both
    /// wear a [`ChoiceButton`]. Counting them would pass whichever panel had
    /// them, which is the one thing this is about.
    fn under_a_sheet(app: &App, entity: Entity) -> bool {
        let mut at = entity;
        loop {
            let found = app.world().entity(at);
            if found.contains::<AbilitySheetRoot>() {
                return true;
            }
            let Some(parent) = found.get::<ChildOf>().map(ChildOf::parent) else {
                return false;
            };
            at = parent;
        }
    }

    /// Every indexed-choice row on the screen, and where it is standing.
    fn choice_rows(app: &mut App) -> Vec<(usize, bool)> {
        let mut q = app.world_mut().query::<(Entity, &ChoiceButton)>();
        let found: Vec<(Entity, usize)> = q
            .iter(app.world())
            .map(|(entity, button)| (entity, button.index))
            .collect();
        found
            .into_iter()
            .map(|(entity, index)| (index, under_a_sheet(app, entity)))
            .collect()
    }

    /// How many panels the drawer is holding.
    fn drawer_panels(app: &mut App) -> usize {
        let mut q = app
            .world_mut()
            .query_filtered::<Option<&Children>, With<ledge::drawer::DrawerRoot>>();
        q.iter(app.world())
            .map(|c| c.map_or(0, bevy::ecs::hierarchy::Children::len))
            .sum()
    }

    /// A seat holding one card it can cast two ways, with **this client's
    /// own** chooser standing open over it.
    ///
    /// `CastMenu` is a `Prompt::CastMode` built one step before the engine
    /// would have built one, while the engine is still holding an ordinary
    /// priority window. Two modes and no card text, because
    /// `choices::cast_label` answers `Normal` and `Alternative` out of
    /// `Phrase` when the printing has not arrived — which is the state this
    /// harness is always in.
    fn duel_casting() -> Duel {
        use baylee_engine::choice::CastModeKind;
        let mut duel = duel_with(false);
        duel.view = Some(
            baylee_client_core::test_support::ViewBuilder::new(2)
                .with_hand(vec![("Fire", 2, 4)])
                .build(),
        );
        crate::rebuild_board(&mut duel);
        duel.cast_menu = Some(crate::CastMenu {
            card: ObjectId::new(4, 0),
            modes: vec![
                crate::castmodes::ReachableMode {
                    kind: CastModeKind::Normal,
                    cost: ManaCost::default(),
                    plan: baylee_client_core::manaplan::Plan::default(),
                },
                crate::castmodes::ReachableMode {
                    kind: CastModeKind::Alternative(0),
                    cost: ManaCost::default(),
                    plan: baylee_client_core::manaplan::Plan::default(),
                },
            ],
            pick: 0,
        });
        duel
    }

    /// The ways of casting a card stand on the card's own sheet, and the
    /// drawer keeps the questions with no card to stand beside.
    ///
    /// The owner's answer of 14.09.2026, and AX step 6c: the cast-mode
    /// chooser is the **same piece of parchment** as the ability chooser. §5
    /// had put the indexed chooser in the drawer and left the ability one
    /// beside the card, and the sentence §5 gave for that — *it belongs to
    /// the card, not to the question* — is just as true of this one.
    ///
    /// The colour half is what stops the sheet swallowing every indexed
    /// choice there is. A colour has no card: the question comes off a mana
    /// ability that is already resolving, so there is nothing on the table to
    /// hang paper beside, and a rule that moved *all* `ChoiceButton`s onto a
    /// sheet would have nowhere to put it.
    #[test]
    fn the_ways_to_cast_a_card_stand_on_its_sheet_and_not_in_the_drawer() {
        let mut app = bar_of(duel_casting());
        let rows = choice_rows(&mut app);
        assert_eq!(
            rows,
            vec![(0, true), (1, true)],
            "both ways of casting the card have to be on the leaf beside it"
        );
        assert_eq!(
            drawer_panels(&mut app),
            0,
            "the rows moved to the sheet, so the drawer is an empty panel \
             standing over the table saying nothing"
        );

        // And the question with no card to stand beside stayed where it was.
        let mut app = bar_of(duel_choosing_a_colour());
        let rows = choice_rows(&mut app);
        assert_eq!(
            rows,
            vec![(0, false), (1, false), (2, false)],
            "a colour is answered out of the drawer — it comes off an ability \
             that is already resolving and has no card to hang paper beside"
        );
        assert_eq!(drawer_panels(&mut app), 1, "and they stand on one panel");
    }

    /// The same question drawn in the same place however it arrived.
    ///
    /// This client asks first, but the engine asks `ChooseCastMode` itself
    /// whenever the client did not get there first — a modal trigger, a
    /// pathway, a seat driven over the wire. Both are `Prompt::CastMode` and
    /// both are about a card, so a chooser that stood beside the card on one
    /// route and in the drawer on the other would be one question moving
    /// depending on how it had arrived.
    ///
    /// It is also the half that fails against the code this replaced: the
    /// drawer stopped reading `Prompt::CastMode` at all, so a sheet that read
    /// only `Duel::cast_menu` would draw this question **nowhere**.
    ///
    /// The cross is the second assertion, and it is the difference the two
    /// routes really do have. Every door out of this sheet works by clearing
    /// the menu that opened it; a question the engine asked is one the table
    /// is waiting on, so there is nothing to clear and a cross there would be
    /// a control that visibly does nothing.
    #[test]
    fn a_cast_question_the_engine_asked_lands_on_the_same_sheet() {
        use baylee_engine::choice::{CastModeDesc, CastModeKind};

        let crosses = |app: &mut App| {
            let mut q = app.world_mut().query_filtered::<Entity, With<SheetClose>>();
            q.iter(app.world()).count()
        };

        let mut duel = duel_casting();
        duel.cast_menu = None;
        duel.interaction = Some(baylee_client_core::Interaction::new(
            baylee_engine::choice::Pending::ChooseCastMode {
                player: PlayerId::new(0),
                object: ObjectId::new(4, 0),
                options: vec![
                    CastModeDesc {
                        index: 0,
                        kind: CastModeKind::Normal,
                        cost: ManaCost::default(),
                    },
                    CastModeDesc {
                        index: 1,
                        kind: CastModeKind::Alternative(0),
                        cost: ManaCost::default(),
                    },
                ],
            },
            PlayerId::new(0),
        ));
        let mut app = bar_of(duel);
        assert_eq!(
            choice_rows(&mut app),
            vec![(0, true), (1, true)],
            "the engine asked the question this time, and it is the same \
             question about the same card"
        );
        assert_eq!(
            drawer_panels(&mut app),
            0,
            "and it is not drawn twice, nor left in the drawer it came from"
        );
        assert_eq!(
            crosses(&mut app),
            0,
            "a question the table is waiting on cannot be put down, so the \
             cross on it would be a door to nowhere"
        );

        // The counter-half: the cross is drawn where there *is* something to
        // clear, or the assertion above would pass on a sheet that never had
        // one.
        let mut app = bar_of(duel_casting());
        assert_eq!(
            crosses(&mut app),
            1,
            "this client's own chooser can be put down, and the cross is how"
        );
    }

    /// A sheet put away stands for as long as its flight into the tray, and
    /// then it is gone.
    ///
    /// Both halves, and the second is what makes the first mean anything: a
    /// dialog that simply stopped being despawned would pass "still standing"
    /// for ever, and the whole reason `sync_tray` hands the sheet to
    /// `reveal_tray` rather than tearing it down is that something else takes
    /// it off the tree at the end.
    ///
    /// Motion is turned back **on** for this test. The harness runs with
    /// `reduce_motion`, where the flight is over on the frame it starts —
    /// which is what keeps every test written before it meaning what it
    /// meant, and which would make this one unable to see the movement at
    /// all.
    #[test]
    fn a_sheet_put_away_flies_to_the_tray_before_it_stops_existing() {
        use std::time::Duration;

        let mut duel = duel_with(false);
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        duel.browser.open();
        let mut app = bar_of(duel);
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = false;

        let bands = |app: &mut App| {
            let mut q = app.world_mut().query_filtered::<Entity, With<TrayBand>>();
            q.iter(app.world()).count()
        };
        assert_eq!(bands(&mut app), 1, "the sheet was opened and is drawn");

        // The player presses the tray button. The browser is shut on this
        // very frame; the sheet is not.
        app.world_mut().resource_mut::<Duel>().browser.close();
        app.update();
        assert_eq!(
            bands(&mut app),
            1,
            "the sheet was despawned on the frame the browser shut, so there \
             is nothing left for the flight to move"
        );

        // A frame that is not long enough, so that "it went" is about the
        // span and not about the next `update` whenever it happens.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(40));
        app.update();
        assert_eq!(bands(&mut app), 1, "it left before its flight was over");

        // And past the end of it.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(400));
        app.update();
        assert_eq!(
            bands(&mut app),
            0,
            "the sheet is still on the tree with nothing left to move it"
        );
    }

    /// A sheet a **question** opened does not fly: it is dismissed where it
    /// stands.
    ///
    /// The other arm of `reveal_tray`'s match, and the one with nowhere to
    /// go — the tray's button is *held* for as long as the question owns the
    /// sheet, so a flight to it would be a movement towards a door the player
    /// was not allowed through. `hud::motion`'s own rule is that a dismissal
    /// is not a journey, and this takes `motion::shutting`, the curve the
    /// drawer is dismissed on.
    ///
    /// Which arm is taken is decided from `TrayReveal::to_tray`, recorded
    /// while the sheet is **up**: by the time it closes the browser is shut,
    /// and a shut browser answers `for_choice` with false. So the `follow(…,
    /// None)` below is doing two jobs — it answers the question, and it is
    /// the only route by which this arm is reachable at all.
    ///
    /// Motion is turned back on for its sibling's reason: under
    /// `reduce_motion` the whole movement happens in the frame it starts and
    /// there is nothing left to read.
    #[test]
    fn an_answered_sheet_is_dismissed_where_it_stands() {
        use baylee_client_core::test_support::ViewBuilder;
        use baylee_engine::choice::{ChoicePrompt, Pending};
        use std::time::Duration;

        let view = ViewBuilder::new(2).build();
        let asked = baylee_client_core::interaction::Interaction::new(
            Pending::ChooseCards {
                player: PlayerId::new(0),
                options: vec![ObjectId::new(7, 0)],
                min: 1,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
            },
            PlayerId::new(0),
        );

        let mut duel = duel_with(false);
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        duel.browser.follow(&view, Some(&asked));
        assert!(
            duel.browser.for_choice(),
            "the harness did not open the sheet for a question"
        );

        let mut app = bar_of(duel);
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = false;

        // Answered: the next question wants nothing from the sheet, which is
        // `Browser::follow`'s one exception to leaving an open sheet alone.
        app.world_mut()
            .resource_mut::<Duel>()
            .browser
            .follow(&view, None);
        app.update();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(40));
        app.update();

        let read: Vec<(Val2, Vec2)> = {
            let mut q = app
                .world_mut()
                .query_filtered::<&UiTransform, With<TrayPanel>>();
            q.iter(app.world())
                .map(|t| (t.translation, t.scale))
                .collect()
        };
        assert_eq!(
            read.len(),
            1,
            "the answered sheet is not on the tree to be read, so nothing \
             below would be a measurement of anything"
        );
        let (translation, scale) = read[0];
        assert_eq!(
            translation,
            Val2::new(px(0.0), px(0.0)),
            "an answered sheet travelled, and it has nowhere to travel to"
        );
        assert!(
            scale.x < 1.0,
            "an answered sheet did not shrink at all: {scale:?}"
        );
    }

    /// A sheet caught on its way out is not turned round: it goes, and a
    /// fresh one opens in its place. Either way there is exactly **one**.
    ///
    /// The count is the assertion. A flight runs on its own span and against
    /// its own curve, so `sync_tray` despawns whatever is leaving rather than
    /// reversing it — and the failure that shape is guarding against is two
    /// sheets on the tree at once, one of them shrinking towards the tray
    /// while the other one draws the rows.
    #[test]
    fn a_sheet_reopened_mid_flight_is_one_sheet_and_not_two() {
        let mut duel = duel_with(false);
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        duel.browser.open();
        let mut app = bar_of(duel);
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = false;

        let bands = |app: &mut App| {
            let mut q = app.world_mut().query_filtered::<Entity, With<TrayBand>>();
            q.iter(app.world()).collect::<Vec<_>>()
        };
        let was = bands(&mut app);
        assert_eq!(was.len(), 1, "one sheet to begin with");

        app.world_mut().resource_mut::<Duel>().browser.close();
        app.update();
        app.world_mut().resource_mut::<Duel>().browser.open();
        app.update();

        let now = bands(&mut app);
        assert_eq!(now.len(), 1, "a sheet was caught and turned round: {now:?}");
        assert!(
            now[0] != was[0],
            "the leaving sheet was kept and refilled, so it is still running \
             the closing curve with the rows of a question that came back"
        );
    }

    /// A maximised sheet offers to **restore**, and a normal one to maximise.
    ///
    /// The mark is chosen at build time from `place.is_maximised(band)`, and
    /// the placement is deliberately not one of `TrayRevision`'s fields — a
    /// drag writes it every frame and rebuilding the sheet per pixel would
    /// make it unusable. So resizing the sheet leaves the head drawn from a
    /// rectangle that is no longer true, and a maximised sheet went on
    /// offering to maximise until something unrelated caused a rebuild.
    /// `TrayRevision::relayout` is the one word that says "the size stopped
    /// changing", and this is the assertion it exists for: without the
    /// `!stale` term in the gate the second half of this test reads the first
    /// half's tree.
    ///
    /// Two marks and not one toggled ink, because a control offering what it
    /// cannot do is the lie this dialog already refuses to tell with a lit
    /// tab or an unlit Confirm.
    #[test]
    fn a_maximised_sheet_offers_to_put_itself_back() {
        use baylee_client_core::browser::Placement;

        let mut duel = duel_with(false);
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        duel.browser.open();
        let mut app = bar_of(duel);

        let marks = |app: &mut App| {
            let said = said(app);
            (
                said.iter().any(|t| t.contains(glyph::MAXIMISE)),
                said.iter().any(|t| t.contains(glyph::RESTORE)),
            )
        };
        assert_eq!(
            marks(&mut app),
            (true, false),
            "a sheet at its opening size offers the wrong thing"
        );

        // The sheet is maximised the way the button maximises it: the store
        // gets the band, and the movement that wrote it says it has stopped.
        // `band_of` has no window here and answers with its own fallback,
        // which is the band this app is laid out in.
        let band = (1280.0, 720.0 - EDGE - hand::HAND_ZONE_H);
        app.world_mut()
            .resource_mut::<crate::settings::ClientSettings>()
            .zone_browser = Some(Placement::maximised(band));
        app.world_mut()
            .resource_mut::<tray::TrayRevision>()
            .relayout();
        app.update();
        assert_eq!(
            marks(&mut app),
            (false, true),
            "the sheet fills the band and its head still offers to fill it"
        );
    }

    /// The end screen's veil is not the zone dialog's, and a rebuild of the
    /// one does not take the other.
    ///
    /// `hud::finish` spawns a `TableVeil` of its own under its own root, and
    /// `sync_tray` used to tear down every `TableVeil` there was — so a game
    /// that ended while anything about the dialog changed lost its
    /// darkening, and `dim_the_table` then had no node to paint. It was never
    /// observed, which is the reason to pin it: the two surfaces are painted
    /// by one system on purpose and owned by two, and only a marker can say
    /// which is which.
    #[test]
    fn the_end_screens_veil_is_not_torn_down_with_the_dialogs() {
        let mut duel = duel_with(false);
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        let mut app = bar_of(duel);

        // Exactly as `hud::finish` does it: the shared constructor, and no
        // `TrayVeil` on top of it.
        let theirs = {
            let mut queue = bevy::ecs::world::CommandQueue::default();
            let id = {
                let mut commands = Commands::new(&mut queue, app.world());
                tray::spawn_veil(&mut commands)
            };
            queue.apply(app.world_mut());
            id
        };

        // Something about the dialog changes, which is what makes `sync_tray`
        // run its teardown at all.
        app.world_mut().resource_mut::<Duel>().browser.open();
        app.update();
        assert!(
            app.world().get_entity(theirs).is_ok(),
            "opening the zone dialog despawned the end screen's veil"
        );

        // And the counter-test: the dialog's own veil *is* torn down, so the
        // assertion above is about the marker and not about a teardown that
        // has quietly stopped happening.
        let mut ours = app.world_mut().query_filtered::<Entity, With<TrayVeil>>();
        let ours: Vec<Entity> = ours.iter(app.world()).collect();
        assert_eq!(ours.len(), 1, "the dialog drew a veil of its own");
        app.world_mut()
            .resource_mut::<Duel>()
            .browser
            .push_filter('a');
        app.update();
        let mut now = app.world_mut().query_filtered::<Entity, With<TrayVeil>>();
        let now: Vec<Entity> = now.iter(app.world()).collect();
        assert!(
            now.len() == 1 && now[0] != ours[0],
            "the dialog's own veil survived its rebuild, so this test would \
             pass on a teardown that despawns nothing at all"
        );
    }

    /// The stack has an answer of its own, and it is only offered while there
    /// is a stack.
    ///
    /// Three mechanisms stand in that row and look alike deliberately — the
    /// engine's `Pass`, an engine hold, and a client-side autopilot — so the
    /// one thing a test can check is that the middle of them appears exactly
    /// when it does something. On an empty stack `hold_action(false)` sends
    /// `UntilStackEmpty { depth: 0 }`, a hold that ends on the frame it
    /// begins.
    ///
    /// The cap it wears is `ledge::a_command_wears_the_key_that_does_the_same
    /// _thing`'s and not this test's: a headless app has no `Window`, so the
    /// shelf is arranged against the 1200-pixel fallback and spends that rung
    /// on the keycaps first — the caps are legitimately absent here.
    #[test]
    fn the_stack_can_be_let_go_of_only_while_there_is_one() {
        let held = Phrase::ResolveTheStack.text(Lang::En).to_string();

        let mut empty = bar_of(duel_saying(false, false));
        let lines = said(&mut empty);
        assert!(
            !lines.contains(&held),
            "a hold until an empty stack is empty promises nothing: {lines:?}"
        );

        let mut app = bar_of(duel_with_a_stack());
        let lines = said(&mut app);
        assert!(
            lines.contains(&held),
            "there is a stack, so there is something to let resolve: {lines:?}"
        );

        // And it is a `MenuButton`, not an answer: the engine was not asked
        // this. `menu_click` is the other half — it re-reads the same
        // predicate before sending.
        let mut q = app
            .world_mut()
            .query_filtered::<&MenuButton, With<crate::ambience::Feel>>();
        let kinds: Vec<MenuAction> = q.iter(app.world()).map(|b| b.action).collect();
        assert!(
            kinds.contains(&MenuAction::HoldForStack),
            "the button carries the deed it does: {kinds:?}"
        );
        let mut prompts = app.world_mut().query::<&PromptButton>();
        let answers: Vec<PromptAction> = prompts
            .iter(app.world())
            .map(|b| b.action)
            .collect::<Vec<_>>();
        assert!(
            answers.contains(&PromptAction::Confirm) && answers.contains(&PromptAction::SkipTurn),
            "and it stands between the two that are answers: {answers:?}"
        );
    }

    /// A seat with nothing to answer: the opponent holds priority.
    ///
    /// The state the mana pool used to vanish in, and the only one in which
    /// the shelf's rule and the chip's rule differ.
    fn duel_watching() -> Duel {
        let mut duel = Duel {
            interaction: Some(baylee_client_core::Interaction::new(
                baylee_engine::choice::Pending::Priority {
                    player: PlayerId::new(1),
                    legal: Box::new(baylee_engine::choice::LegalActions::default()),
                },
                PlayerId::new(0),
            )),
            ..Duel::default()
        };
        duel.view = Some(baylee_client_core::test_support::ViewBuilder::new(2).build());
        crate::rebuild_board(&mut duel);
        assert!(!duel.is_my_turn_to_act(), "this seat is watching");
        duel
    }

    /// The mana pool is a badge again, and this time on purpose.
    ///
    /// The turn is the point of the test, so it is written down rather than
    /// swapped out. The chip this began as was drawn "while the seat has
    /// something to answer" and blinked out at every opponent's priority,
    /// which AX §4.1 called movement carrying no information; the column that
    /// replaced it stood always, on the argument that the shelf's left edge
    /// was reserved whatever was on it. Off the shelf that argument has
    /// nothing left to rest on, and the owner asked for the other rule back
    /// on its own terms — *"Es ist hidden, wenn kein Mana im Mana Pool ist
    /// und ist nur dann sichtbar, wenn dort Mana drin ist"*. What is not the
    /// chip's rule is the **condition**: it comes and goes with the *mana*
    /// and not with whose priority it is, so a watching seat holding mana
    /// still sees it, which is the case this test opens with.
    ///
    /// Then the two claims that did not turn: mana in it is a numeral beside
    /// a disc rather than a row of discs to count, and a restricted mana is
    /// an entry of its own (CR 106.6).
    #[test]
    fn the_mana_pool_is_drawn_only_while_something_is_floating() {
        let label = Phrase::ManaPool.text(Lang::En).to_string();

        let mut watching = bar_of(duel_watching());
        assert!(
            !strip_is_shown(&mut watching),
            "nothing is floating, so there is no strip to read"
        );
        assert!(
            !said(&mut watching).contains(&"\u{2014}".to_string()),
            "the em dash the strip replaces is gone, not merely hidden"
        );

        // And with mana in it the strip is up and the count is a numeral.
        let mut duel = duel_watching();
        {
            let view = duel.view.as_mut().expect("the seat has a view");
            let seat = view.seat;
            let pool = &mut view
                .seats
                .iter_mut()
                .find(|s| s.player == seat)
                .expect("this seat sits at its own table")
                .mana_pool;
            pool.green = 3;
            pool.restricted[baylee_core::mana::ManaColor::White.index()] = 1;
        }
        crate::rebuild_board(&mut duel);
        let mut floating = bar_of(duel);
        assert!(
            strip_is_shown(&mut floating),
            "this seat is still only watching, and it is holding mana: the \
             strip follows the pool and not the priority"
        );
        let lines = said(&mut floating);
        assert!(
            lines.contains(&label),
            "the strip says what it is once it is up: {lines:?}"
        );
        assert!(
            lines.contains(&"\u{00d7}3".to_string()) && lines.contains(&"\u{00d7}1".to_string()),
            "three green and one restricted white, as numerals: {lines:?}"
        );
    }

    /// Whether the mana pool's strip is drawn at all.
    fn strip_is_shown(app: &mut App) -> bool {
        let mut q = app
            .world_mut()
            .query_filtered::<&Visibility, With<ledge::pool::PoolStrip>>();
        q.iter(app.world()).any(|seen| *seen != Visibility::Hidden)
    }

    /// The shelf stands off the table above it and off the hand below it, and
    /// it leans on the hand the more lightly of the two.
    ///
    /// The owner asked for the second cast on 14.09.2026 — "zum Tisch hin als
    /// auch zur Hand hin (zur Hand etwas leichter)". It is read off the
    /// **built nodes** and not off the constants, because the constants being
    /// right is not the claim: one cast written twice would satisfy every
    /// number in `ledge.rs` and still be a lid.
    ///
    /// They stopped being a `BoxShadow` when the shelf stopped being opaque,
    /// and the second half of this test is why: a `BoxShadow` is the node's
    /// own rectangle offset and blurred, so both casts lay most of their
    /// weight *inside* the shelf — invisible behind an opaque one, and about
    /// two thirds black across a translucent one. So each cast has to fall
    /// wholly outside the shelf's box, and that is asserted rather than
    /// assumed.
    #[test]
    fn the_shelf_casts_both_ways_and_more_softly_onto_the_hand() {
        let mut app = bar_of(duel_with(false));
        let shelf = app
            .world_mut()
            .query_filtered::<Entity, With<ledge::LedgeShelf>>()
            .iter(app.world())
            .next()
            .expect("the shelf is built");
        let children: Vec<Entity> = app
            .world()
            .entity(shelf)
            .get::<Children>()
            .expect("the shelf has children")
            .iter()
            .collect();
        let px = |v: Val| match v {
            Val::Px(p) => p,
            other => panic!("a cast is measured in pixels, not {other:?}"),
        };
        let casts: Vec<(f32, f32, f32)> = children
            .into_iter()
            .filter_map(|child| {
                // A cast is the only child of the shelf that is a gradient;
                // the three columns are laid out and carry no paint at all.
                let gradient = app.world().entity(child).get::<BackgroundGradient>()?;
                let node = app.world().entity(child).get::<Node>()?;
                let (top, height) = (px(node.top), px(node.height));
                let weight = gradient.0.iter().fold(0.0_f32, |most, g| match g {
                    Gradient::Linear(l) => l
                        .stops
                        .iter()
                        .fold(most, |m, stop| m.max(stop.color.alpha())),
                    _ => most,
                });
                Some((top, height, weight))
            })
            .collect();
        let up = casts
            .iter()
            .find(|(top, _, _)| *top < 0.0)
            .expect("nothing is cast onto the table");
        let down = casts
            .iter()
            .find(|(top, _, _)| *top > 0.0)
            .expect("nothing is cast onto the hand");
        assert!(
            down.2 < up.2,
            "the hand's side is the lighter one: {} against {}",
            down.2,
            up.2
        );
        assert!(down.2 > 0.0, "and it is still a cast, not an absence");
        // Wholly outside the shelf, both of them: the one above ends where
        // the shelf begins, the one below begins where the shelf ends.
        //
        // **Both insets are read from the shelf's padding box**, which is
        // where this assertion used to be wrong in exactly the way the code
        // was. An absolutely-positioned child's `top` is measured from its
        // containing block's padding box, and the shelf carries
        // `border: UiRect::top(px(LIP))` — so `top: -LIFT_UP_H` put the
        // gradient's darkest end one pixel *inside* the bar, on the one line
        // the cloth paints, and `up.0 + up.1 <= 0.0` was satisfied by the
        // overlap rather than in spite of it. Converting to the border box is
        // one addition, and it is the whole of what the claim is about.
        let lip = crate::hud::LEDGE_LIP;
        assert!(
            up.0 + up.1 + lip <= 0.0,
            "the table's cast reaches {} pixels into the shelf",
            up.0 + up.1 + lip
        );
        assert!(
            down.0 + lip >= crate::hud::LEDGE_H,
            "the hand's cast starts {} pixels above the shelf's lower edge",
            crate::hud::LEDGE_H - (down.0 + lip)
        );
        assert!(
            app.world().entity(shelf).get::<BoxShadow>().is_none(),
            "a `BoxShadow` on a translucent shelf lays its own rectangle \
             across the whole width of the window"
        );
    }

    /// A mana that is floating is one entity for as long as it is floating.
    ///
    /// The whole reason `ledge/pool.rs` exists, and the one claim that cannot
    /// be made about anything else on this shelf. §4.1 wants a new entry to
    /// pop and a spent one to fade, and neither is possible while the thing
    /// being animated is despawned and rebuilt whenever the *sentence*
    /// changes — which is at every priority. So: the same entity across a
    /// rebuild that changed both the question and the count, and no entity at
    /// all once the mana is spent.
    ///
    /// The counter-test is in the middle of it. The shelf really is rebuilt
    /// between the two readings — the sentence is a different sentence — so
    /// an entry that survives is surviving something, rather than sitting in
    /// a tree nothing touched.
    #[test]
    fn a_floating_mana_is_one_entity_for_as_long_as_it_floats() {
        let mut app = bar_of(pool_of(1, "one"));
        let first = pool_entries(&mut app);
        assert_eq!(first.len(), 1, "one colour is floating, so one entry");
        let before = said(&mut app).join("|");

        *app.world_mut().resource_mut::<Duel>() = pool_of(2, "two");
        app.update();
        assert_eq!(
            pool_entries(&mut app),
            first,
            "the same mana, more of it: the entry is written, not replaced"
        );
        assert_ne!(
            said(&mut app).join("|"),
            before,
            "this test's premise is that the shelf was rebuilt under it"
        );

        // Spent. With motion off the fade is over on the frame it starts.
        *app.world_mut().resource_mut::<Duel>() = pool_of(0, "three");
        app.update();
        assert!(
            pool_entries(&mut app).is_empty(),
            "a spent mana leaves, rather than being left behind"
        );
        // And the strip waits for it: one more frame, because it can only go
        // away once nothing is standing on it.
        app.update();
        assert!(
            !strip_is_shown(&mut app),
            "the strip goes with the last pip, rather than standing empty"
        );
    }

    /// And it survives the *other* rebuild, which is the one the strip moved
    /// under.
    ///
    /// The test above changes the sentence, which rebuilds the shelf; this
    /// one moves the pointer, which is what [`HudRevision`] counts and what
    /// tears down every child of the root. While the pool was a column on the
    /// shelf those were the same claim made twice, and the exemption that
    /// mattered was `sync_ledge`'s. The strip hangs off the root now, so the
    /// exemption that matters is `sync_overlay`'s own sweep — a different
    /// list, in a different file, that nothing else here would notice the
    /// absence of: a pool rebuilt on every pointer move still *draws*, and
    /// only the pop and the fade are lost, which no test that reads the row
    /// can see.
    ///
    /// The second assertion is the counter-test, and it is the same one the
    /// sweep's own test makes: the root really did rebuild between the two
    /// readings, so an entry that came through came through something.
    #[test]
    fn a_floating_mana_survives_the_pointer_crossing_a_card() {
        let mut app = bar_of(pool_of(1, "one"));
        let first = pool_entries(&mut app);
        assert_eq!(first.len(), 1, "one colour is floating, so one entry");

        // The counter-test, hung under the root by hand. A watching seat with
        // one mana and nothing else draws *nothing* of its own up there — the
        // hand, the stack and the preview are all absent — so "the nodes that
        // were rebuilt" is an empty list on both sides of the update and
        // proves nothing at all. A node the sweep has never been told about
        // is the witness instead: it is gone afterwards exactly when the
        // sweep ran, which is the premise, and the strip beside it came
        // through the same sweep, which is the claim.
        let root = {
            let mut q = app.world_mut().query_filtered::<Entity, With<HudRoot>>();
            q.single(app.world()).expect("one overlay root")
        };
        let decoy = app.world_mut().spawn(Node::default()).id();
        app.world_mut().entity_mut(root).add_child(decoy);

        app.world_mut().resource_mut::<Duel>().hovered = Some(ObjectId::new(1, 0));
        app.update();

        assert!(
            app.world().get_entity(decoy).is_err(),
            "this test's premise is that the root was swept under it"
        );
        assert_eq!(
            pool_entries(&mut app),
            first,
            "the pointer moved and the mana was rebuilt with the overlay, so \
             the pop and the fade have nothing left to animate"
        );
    }

    /// A watching seat with `green` green mana floating and `prompt` on the
    /// shelf.
    ///
    /// The sentence is a parameter because the pool's tests all need to be
    /// able to rebuild the shelf *without* touching the pool, which is the
    /// premise the retained strip exists to survive.
    fn pool_of(green: u16, prompt: &str) -> Duel {
        let mut duel = duel_watching();
        duel.last_error = Some(baylee_client_core::i18n::Refusal::Verbatim(
            prompt.to_string(),
        ));
        {
            let view = duel.view.as_mut().expect("the seat has a view");
            let seat = view.seat;
            view.seats
                .iter_mut()
                .find(|s| s.player == seat)
                .expect("this seat sits at its own table")
                .mana_pool
                .green = green;
        }
        crate::rebuild_board(&mut duel);
        duel
    }

    fn pool_entries(app: &mut App) -> Vec<Entity> {
        let mut q = app.world_mut().query::<(Entity, &ledge::pool::PoolEntry)>();
        q.iter(app.world()).map(|(e, _)| e).collect::<Vec<_>>()
    }

    /// The strip grows out of the shelf rather than appearing on it.
    ///
    /// Three claims, and the first two are what a plain `from_bottom` would
    /// break. On the frame the first mana arrives the strip is drawn at
    /// [`motion::ZOOM_FROM`] — the start of the arrival, not the end of it —
    /// and it is pinned at the **bottom-left** corner, because a node fixed
    /// at the left margin that shrinks toward its own middle slides right as
    /// it grows. Then the movement ends: the clock is advanced past
    /// [`motion::ZOOM_IN`] and the strip is at full size, which is what makes
    /// the first two an arrival rather than a strip permanently drawn 12%
    /// short.
    ///
    /// With `reduce_motion` it is at full size on that same first frame,
    /// which is the counter-test for the first claim on its own terms.
    #[test]
    fn the_strip_grows_out_of_the_shelf_rather_than_appearing_on_it() {
        for (still, want) in [(false, motion::ZOOM_FROM), (true, 1.0)] {
            let mut app = bar_of(pool_of(0, "none"));
            app.world_mut()
                .resource_mut::<crate::prefs::Prefs>()
                .edit()
                .reduce_motion = still;
            assert!(!strip_is_shown(&mut app), "nothing floating yet");

            *app.world_mut().resource_mut::<Duel>() = pool_of(1, "one");
            app.update();

            assert!(
                strip_is_shown(&mut app),
                "a mana arrived, so the strip is up"
            );
            let (scale, shift) = strip_pose(&mut app);
            assert!(
                (scale.x - want).abs() < 0.001 && (scale.y - want).abs() < 0.001,
                "reduce_motion {still}: the strip is drawn at {scale:?} and \
                 {want} was wanted"
            );
            if !still {
                assert_eq!(
                    shift,
                    motion::from_bottom_left(want),
                    "the strip grows out of the corner it is pinned at"
                );

                tick(&mut app, motion::ZOOM_IN + 0.01);
                let (scale, shift) = strip_pose(&mut app);
                assert!(
                    (scale.x - 1.0).abs() < 0.001,
                    "and the arrival ends at full size, not at {scale:?}"
                );
                assert_eq!(
                    shift,
                    motion::from_bottom_left(1.0),
                    "with nothing left to correct for"
                );
            }
        }
    }

    /// And it folds back into the shelf rather than being taken off it.
    ///
    /// The clock has to be advanced for this one, and that is the finding
    /// rather than a detail. Written against the harness's own still clock it
    /// read as a test and asserted nothing: the strip's fold cannot start
    /// until the row is empty, the last pip cannot finish fading while
    /// `delta` is zero, so with the movement on the fold was never reached
    /// and "the strip is still shown" was true for the wrong reason. It
    /// passed against a `grow_the_pool` that hid the strip the instant the
    /// pool emptied — the exact fault it was written for.
    ///
    /// So the two movements are walked through in order, each with the clock
    /// pushed past its own span: the pip leaves, the frame after that reads
    /// an empty row and starts the fold, and the strip is **still there**
    /// through it. Only past [`motion::ZOOM_OUT`] again is it away.
    #[test]
    fn the_strip_folds_back_into_the_shelf_rather_than_being_taken_off_it() {
        let mut app = bar_of(pool_of(1, "one"));
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = false;
        app.update();
        assert!(strip_is_shown(&mut app), "one mana is floating");

        // Spent. The pip's own fade runs first, because the strip must not
        // fold around something still standing on it.
        *app.world_mut().resource_mut::<Duel>() = pool_of(0, "none");
        tick(&mut app, motion::ZOOM_OUT + 0.01);
        assert!(pool_entries(&mut app).is_empty(), "the pip has gone");
        assert!(
            strip_is_shown(&mut app),
            "and the strip is still up: nothing has read the empty row yet"
        );

        // The frame that reads it and starts the fold, with no time in it —
        // so the strip is at the beginning of its own movement and not past
        // the end of it.
        tick(&mut app, 0.0);
        assert!(
            strip_is_shown(&mut app),
            "the strip folds away rather than being taken away"
        );
        let (scale, _) = strip_pose(&mut app);
        assert!(
            (scale.x - 1.0).abs() < 0.001,
            "and the fold begins at full size, not at {scale:?}"
        );

        tick(&mut app, motion::ZOOM_OUT + 0.01);
        assert!(
            !strip_is_shown(&mut app),
            "and once the fold is over the strip is away"
        );
    }

    /// One frame, with `seconds` of clock in front of it.
    fn tick(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(seconds));
        app.update();
    }

    /// The strip's scale and where that scale is anchored.
    fn strip_pose(app: &mut App) -> (Vec2, Val2) {
        let mut q = app
            .world_mut()
            .query_filtered::<&UiTransform, With<ledge::pool::PoolStrip>>();
        let at = q.single(app.world()).expect("one strip");
        (at.scale, at.translation)
    }

    /// The strip never goes away around a pip that is still on it.
    ///
    /// The same claim this made about the em dash, on the thing that replaced
    /// it — and it got *sharper* in the substitution, which is why the test
    /// stayed. A dash drawn over a fading pip was a second reading of the row
    /// beside the first; a strip taken away over one deletes the fade
    /// outright, so the pip the player spent vanishes instead of leaving.
    ///
    /// The bug it guards is one missing clause. On the spend frame the pool
    /// names nothing and nothing is *yet* marked closing, so a reading that
    /// asks only "is anything fading" answers "the row is empty" on the one
    /// frame where the row is at its fullest.
    ///
    /// It needs the movement **on**, which is why it is a second test rather
    /// than two more lines in the one above. With `reduce_motion` the spent
    /// pip is despawned before the frame ends, so a strip gone beside it is
    /// right by accident; here the harness clock never advances, so the
    /// fading entry sits at the start of its fade for as long as the test
    /// looks at it. The first assertion is the counter-test: the pip really
    /// is still there, so the strip that stayed up is the strip *waiting*
    /// rather than a row that was never emptied.
    #[test]
    fn the_strip_does_not_go_away_over_a_pip_that_is_still_fading() {
        let mut app = bar_of(pool_of(1, "one"));
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = false;

        *app.world_mut().resource_mut::<Duel>() = pool_of(0, "two");
        app.update();
        assert_eq!(
            pool_entries(&mut app).len(),
            1,
            "the spent mana is still on the row, fading"
        );
        assert!(
            strip_is_shown(&mut app),
            "so the strip must not be taken out from under it"
        );
    }

    // ---- the slip under the preview -------------------------------------
    //
    // `hud::slip` is tested on its own arithmetic and on its own system;
    // what those cannot say is whether `sync_overlay` ever builds one.
    // "Declared but never wired" is a bug this client has shipped before.

    /// A duel with one ability on the stack, its source on the battlefield,
    /// and the pointer on the stack entry.
    fn hovering_the_stack(hovered: bool) -> (Duel, crate::cardtext::CardTexts) {
        use baylee_client_core::card_face::{CardTextEntry, FaceText};
        use baylee_client_core::test_support::{ViewBuilder, printed, statics, token};

        let texts = crate::cardtext::CardTexts::filed(
            baylee_core::ids::PrintRef::new(7),
            CardTextEntry {
                scryfall_id: "abc".to_string(),
                lang: "de".to_string(),
                faces: vec![FaceText {
                    name: "Ondu-Kleriker".to_string(),
                    english_name: "Ondu Cleric".to_string(),
                    type_line: "Kreatur".to_string(),
                    oracle_text: "Ziehe eine Karte.".to_string(),
                    mana_cost: String::new(),
                }],
            },
        );
        let mut ability = token(30, 0, "Ondu Cleric", 0, 0);
        ability.card = None;
        ability.stack_item = Some(baylee_view::StackItem::Ability {
            source: ObjectId::new(7, 0),
            ability: None,
            text: Some(baylee_view::StackText {
                face: 0,
                line: 0,
                of: 1,
            }),
        });
        let mut duel = Duel {
            interaction: Some(baylee_client_core::Interaction::new(
                baylee_engine::choice::Pending::Priority {
                    player: PlayerId::new(0),
                    legal: Box::new(baylee_engine::choice::LegalActions::default()),
                },
                PlayerId::new(0),
            )),
            statics: Some(statics(8)),
            hovered: hovered.then(|| ObjectId::new(30, 0)),
            ..Duel::default()
        };
        duel.view = Some(
            ViewBuilder::new(2)
                .with_battlefield(0, vec![printed(7, 0, "Ondu Cleric", 7)])
                .with_stack(vec![ability])
                .build(),
        );
        crate::rebuild_board(&mut duel);
        (duel, texts)
    }

    fn overlay_with(duel: Duel, texts: crate::cardtext::CardTexts) -> App {
        // The stack panel draws pictures, and asking for one is an
        // `AssetServer::load` — which spawns on the IO pool and panics
        // without it. Idempotent, so several tests may ask.
        bevy::tasks::IoTaskPool::get_or_init(Default::default);
        let mut app = bar_of(duel);
        app.insert_resource(texts);
        app.world_mut().resource_mut::<HudRevision>().set_changed();
        app.update();
        app
    }

    /// The whole point of the feature, end to end: the pointer is on a stack
    /// entry, and the sentence the row abbreviates is written out under the
    /// preview.
    #[test]
    fn hovering_a_stack_entry_writes_its_sentence_out() {
        let (duel, texts) = hovering_the_stack(true);
        let mut app = overlay_with(duel, texts);
        let said = said(&mut app);
        // Three, and the third is the interesting one. The row says it cut
        // to four lines; the sheet says it whole; and the preview's *card*
        // says it too, because no art has arrived in a headless test and a
        // card with no picture falls back to a constructed face. That third
        // one is exactly why `slip::says` asks `FaceCtx::always` — the
        // player's own choice — rather than "did a face come back": the
        // fallback is the ordinary case offline, and suppressing the sheet
        // there would silence it wherever a gateway serves no art.
        assert_eq!(
            said.iter()
                .filter(|line| line.contains("Ziehe eine Karte."))
                .count(),
            3,
            "the row, the fallback face and the sheet: {said:?}"
        );
        let mut sheets = app
            .world_mut()
            .query_filtered::<Entity, With<super::super::slip::Washing>>();
        assert!(
            sheets.iter(app.world()).count() > 0,
            "and it is on a sheet the wash can reach"
        );
        // And the sheet is *in* the bubble. A slip spawned and never added to
        // anything is still in the world, still carries its `Washing`, and
        // still answers `said` — so without this the test passes on a sheet
        // nobody can see.
        let mut orphans = app
            .world_mut()
            .query_filtered::<Entity, (With<super::super::slip::Washing>, Without<ChildOf>)>();
        assert_eq!(
            orphans.iter(app.world()).count(),
            0,
            "a sheet hanging off nothing is drawn nowhere"
        );
    }

    /// The counter-test, without which the one above would pass on a client
    /// that drew a sheet under every preview it ever opened.
    #[test]
    fn a_pointer_on_nothing_opens_no_sheet() {
        let (duel, texts) = hovering_the_stack(false);
        let mut app = overlay_with(duel, texts);
        let said = said(&mut app);
        // Once, and it is the row's own: the panel abbreviates whether
        // anyone is looking or not, and the sheet is what the looking buys.
        assert_eq!(
            said.iter()
                .filter(|line| line.contains("Ziehe eine Karte."))
                .count(),
            1,
            "the row says it and nothing else does: {said:?}"
        );
        let mut sheets = app
            .world_mut()
            .query_filtered::<Entity, With<super::super::slip::Washing>>();
        assert_eq!(sheets.iter(app.world()).count(), 0);
    }

    /// A stack entry outlives the game it belonged to.
    ///
    /// The owner conceded as a function test with one of Sheoldred's
    /// triggers on the stack, and the end screen came up with the trigger
    /// still drawn beside it — an entry whose whole job is to say that
    /// something is about to happen, in a game where nothing will. The
    /// prompt bar has stopped *whole* at `GameOver` for exactly this reason,
    /// and the stack panel was the fourth reader of `Duel::ending` that
    /// nobody had connected.
    #[test]
    fn the_stack_panel_stops_with_the_prompt_bar_when_the_game_is_over() {
        let (duel, texts) = hovering_the_stack(false);
        let mut app = overlay_with(duel, texts);
        let running = said(&mut app);
        assert!(
            running.iter().any(|line| line.contains("Ondu")),
            "the premise: a running game draws the stack it has: {running:?}"
        );

        let (mut duel, texts) = hovering_the_stack(false);
        duel.interaction = Some(baylee_client_core::Interaction::new(
            baylee_engine::choice::Pending::GameOver(GameResult {
                winner: Some(Victor::Player(PlayerId::new(0))),
                reason: EndReason::LastPlayerStanding,
            }),
            PlayerId::new(0),
        ));
        crate::rebuild_board(&mut duel);
        let mut app = overlay_with(duel, texts);
        let over = said(&mut app);
        assert!(
            !over.iter().any(|line| line.contains("Ondu")),
            "the same stack is still drawn under a finished game: {over:?}"
        );
    }

    /// The stack's head names the seat the table is waiting for, and it is
    /// the seat the **engine asked** rather than the one holding priority.
    ///
    /// `waiting_line` has been tested since it was written, and that test
    /// says nothing at all about this: it is a pure function over a name, so
    /// it passed just as well while `spawn_stack_panel` fed it a seat the
    /// host had taken from `priority_holder`. A seat asked to declare
    /// blockers or to discard holds no priority, so the head fell silent on
    /// exactly the questions a player most needs pointing at — the defect
    /// #81 is about, one surface further along than the caret.
    ///
    /// So this one runs the system and reads the words off the tree. The
    /// counter-half is the seat nobody is being asked: the line is absent
    /// once the game is over, which is the only question the engine asks
    /// nobody, and its absence is what proves the assertion is reading this
    /// line rather than some constant of the panel's.
    #[test]
    fn the_stack_says_which_seat_the_table_is_waiting_for() {
        let named = |awaiting: Option<u8>| {
            let mut duel = duel_with_a_stack();
            let view = duel.view.as_mut().expect("the seat has a view");
            view.awaiting = awaiting.map(PlayerId::new);
            // The default roster seats only the viewing player, and the
            // line under test names somebody else.
            let mut statics = baylee_client_core::test_support::statics(8);
            statics.seats.push(baylee_view::SeatIdentity {
                player: PlayerId::new(1),
                display_name: "sharp 1".to_string(),
                is_ai: true,
                away: false,
                team: None,
            });
            duel.statics = Some(statics);
            crate::rebuild_board(&mut duel);
            said(&mut overlay_with(
                duel,
                crate::cardtext::CardTexts::default(),
            ))
        };

        let opponent = named(Some(1));
        assert!(
            opponent.contains(&"waiting for sharp 1".to_string()),
            "the head names the seat being asked: {opponent:?}"
        );

        let me = named(Some(0));
        assert!(
            me.contains(&Phrase::WaitingForYou.text(Lang::En).to_string()),
            "and addresses this seat rather than naming it: {me:?}"
        );

        let nobody = named(None);
        assert!(
            !nobody.iter().any(|line| line.starts_with("waiting for")),
            "nothing is asked of anyone, so the head says nothing: {nobody:?}"
        );
    }
}
