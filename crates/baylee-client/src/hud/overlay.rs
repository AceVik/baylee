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
    // Generated at startup, so a headless app that never ran `setup_sheets`
    // simply draws the flat colour the sheet is grained around.
    sheets: Option<Res<UiSheets>>,
) {
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
        .map(|m| m.prompt().headline(lang, turn, duel.statics.as_ref()))
        .or_else(|| {
            duel.interaction
                .as_ref()
                .filter(|_| !over)
                .map(|i| i.prompt().headline(lang, turn, duel.statics.as_ref()))
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
    let browser = BrowserGate {
        open: duel.browser.is_open(),
        tab: duel.browser.tab(),
        filter: duel.browser.filter().to_string(),
        typing: duel.browser.is_typing(),
        sort: duel.browser.sort(),
        descending: duel.browser.descending(),
    };
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
        && revision.browser == browser
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
    revision.browser = browser;
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
            if !tree.shelf.contains(*child) && !tree.drawer.contains(*child) {
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
        let ledge = ledge::spawn_ledge(&mut commands);
        commands.entity(root).add_child(ledge);
        // The pool's column is the shelf's one retained child — it is where a
        // mana arriving is drawn arriving, and `sync_ledge` despawns the
        // other two on every rebuild. See [`ledge::pool`].
        let pool = ledge::pool::spawn_pool_column(&mut commands);
        commands.entity(ledge).add_child(pool);
        // And the drawer's node, for the same two reasons: it outlives every
        // rebuild this system does, and what fills it is not any of this
        // system's business. See [`ledge::drawer`].
        let drawer = ledge::drawer::spawn_drawer_root(&mut commands);
        commands.entity(root).add_child(drawer);
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
        let layout = hand_layout(board.hand.len(), HAND_CARD_W, available);
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
            &mut textures,
            &assets,
            &fonts,
            &faces,
            &motion.sheen,
            &motion.touch,
            cards.as_mut(),
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
            let place = preview_place(anchor, panel, window);
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
            let (art, look) = match far_face(key, two_faced(view, hovered)) {
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
                    sheets.as_deref(),
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
    if let (false, Some(statics)) = (board.stack.is_empty(), duel.statics.as_ref()) {
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
    // Opened from the table (a tap on the top card of a pile), from the
    // keyboard, or by the engine asking a question about cards the table
    // cannot show. Nothing draws a second copy of a zone to click.
    if let (true, Some(statics)) = (duel.browser.is_open(), duel.statics.as_ref()) {
        // Where the sheet stands, decided by the browser so that the tray
        // takes a rectangle rather than the window and the store. `fit` is
        // applied on every build and never written back: a window briefly
        // dragged narrow must not overwrite where the player put the sheet on
        // the screen they play on. A sheet a *question* opened reads no store
        // at all and is centred — `Browser::placement` carries the
        // measurement that says why a clamp was not enough.
        // W2: the table goes dark behind a dialog that holds the whole answer,
        // and behind no other — `Browser::dims_the_table` carries the argument
        // for why that is a narrower question than "a question opened this".
        //
        // The node is spawned whenever the *sheet* is, and it is
        // `dim_the_table` that decides how dark it is: a question answered by
        // a second one that the sheet only partly holds leaves the sheet
        // standing with its lock gone, and a veil that was spawned on the lock
        // would vanish there instead of lifting. Clear, it is one node
        // painting nothing and answering nothing.
        let veil = tray::spawn_veil(&mut commands);
        commands.entity(root).add_child(veil);
        let band = tray::band_of(&windows);
        let place = duel.browser.placement(band, settings.zone_browser);
        let tray = tray::spawn_tray(
            &mut commands,
            lang,
            &duel.browser,
            view,
            duel.interaction.as_ref(),
            statics,
            &mut textures,
            &assets,
            &fonts,
            &faces,
            cards.as_mut(),
            place,
        );
        commands.entity(root).add_child(tray);
    }
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
/// `two_faced` is asked separately because having a *key* says nothing about
/// how many faces the card has: every printing has one, and a card the seat
/// may not see has none while still being turnable — over to the back, which
/// is exactly what everyone else at the table is looking at.
pub(super) const fn far_face(key: Option<ImageKey>, two_faced: bool) -> Option<ImageKey> {
    match key {
        Some(key) if two_faced => Some(ImageKey {
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

/// Whether the hovered object is a card printed on both sides.
///
/// The view says which face is up, not how many there are, so the answer
/// comes from the registry the client already links for ability labels and
/// mana sources. A token or a face-down permanent has no card and therefore
/// no back.
///
/// This doc had been sitting six items further up, above a `pick_hint` that
/// had its own, since whichever splice put it there — the shape
/// `doc-comment-splice-beheads-the-next-item` is named for. It came back when
/// `pick_hint` went to the drawer and left it standing over nothing.
fn two_faced(view: &PlayerView, hovered: Option<ObjectId>) -> bool {
    hovered
        .and_then(|id| view.object(id))
        .and_then(|object| object.card.as_ref())
        .and_then(|card| baylee_cards::by_index(card.index))
        .is_some_and(|def| def.faces.len() > 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;
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

    /// A headless app that really runs [`sync_overlay`].
    ///
    /// Not a source-reading test, because the claim is about what the system
    /// *builds* rather than about a component only a renderer creates. The
    /// two optional material resources are left out on purpose — that is the
    /// branch a machine with no GPU takes, and it is the branch that draws
    /// the prose this test reads.
    fn bar_of(duel: Duel) -> App {
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
            // All six, chained, in the order the app runs them: the first
            // spawns the shelf, the pool's retained column and the drawer's
            // node, the second writes the shelf and records where its middle
            // ended up, then the drawer is filled over that middle and opened
            // or shut, and the pool's row is reconciled and moved. A harness
            // that ran only the rebuild would be reading a bar with no words
            // on it and calling that an answer.
            .add_systems(
                Update,
                (
                    sync_overlay,
                    ledge::sync_ledge,
                    ledge::drawer::sync_drawer,
                    ledge::drawer::zoom_the_drawer,
                    ledge::pool::sync_pool,
                    ledge::pool::zoom_the_pool,
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
    fn said(app: &mut App) -> Vec<String> {
        let mut roots = app.world_mut().query::<&Text>();
        let mut lines: Vec<String> = roots.iter(app.world()).map(|t| t.0.clone()).collect();
        let mut spans = app.world_mut().query::<&TextSpan>();
        lines.extend(spans.iter(app.world()).map(|s| s.0.clone()));
        lines
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
            last_error: Some(REFUSED.to_string()),
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
    fn duel_with_a_stack() -> Duel {
        let mut duel = duel_saying(false, false);
        let view = duel.view.as_mut().expect("the seat has a view");
        view.stack = vec![baylee_client_core::test_support::token(9, 1, "Shock", 0, 0)];
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
        for pill in [Phrase::OfferADraw, Phrase::Concede] {
            let label = pill.text(Lang::En).to_string();
            assert!(
                lines.contains(&label),
                "every game still being played offers both of these: {lines:?}"
            );
        }
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
    fn the_shelf_and_the_drawer_outlive_a_rebuild_and_nothing_else_does() {
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
        let standing = |app: &mut App| {
            let mut q = app
                .world_mut()
                .query_filtered::<&Children, With<ledge::LedgeShelf>>();
            q.iter(app.world())
                .flat_map(|c| c.iter().collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };
        let drawer = |app: &mut App| {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<ledge::drawer::DrawerRoot>>();
            q.iter(app.world()).collect::<Vec<_>>()
        };
        // Everything the root carries **except** the two nodes the sweep is
        // told to pass over, which is the honest way to name "what the
        // rebuild rebuilds": it says which entities it means rather than
        // resting on a component that happens to be drawn in one place —
        // `MenuButton` was that component and stopped being it the moment the
        // ways out of a game moved to the shelf.
        let redrawn = |app: &mut App| {
            let kept = {
                let mut q = app.world_mut().query_filtered::<
                    Entity,
                    Or<(With<ledge::LedgeShelf>, With<ledge::drawer::DrawerRoot>)>,
                >();
                q.iter(app.world()).collect::<Vec<_>>()
            };
            let mut q = app.world_mut().query_filtered::<&Children, With<HudRoot>>();
            q.iter(app.world())
                .flat_map(|c| c.iter().collect::<Vec<_>>())
                .filter(|e| !kept.contains(e))
                .collect::<Vec<_>>()
        };

        let was_shelf = shelf(&mut app);
        let was_drawer = drawer(&mut app);
        let was_root = roots(&mut app);
        let was_standing = standing(&mut app);
        let was_redrawn = redrawn(&mut app);
        assert_eq!(was_shelf.len(), 1, "one shelf, and it was built");
        assert_eq!(was_drawer.len(), 1, "and one drawer beside it");
        assert_eq!(was_root.len(), 1, "and one root to hang them off");
        assert_eq!(was_standing.len(), 3, "and three columns standing on it");
        assert!(
            !was_redrawn.is_empty(),
            "the overlay drew something of its own beside the two"
        );

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
        assert_eq!(
            standing(&mut app),
            was_standing,
            "the shelf kept its place and lost what was on it, which is the \
             same loss one level down: a `Feel` under the pointer goes back \
             to rest"
        );
        let now_redrawn = redrawn(&mut app);
        assert!(!now_redrawn.is_empty(), "the overlay still draws it");
        assert!(
            now_redrawn.iter().all(|e| !was_redrawn.contains(e)),
            "the overlay stopped rebuilding: it is still showing the tree it \
             built for a different frame"
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

    /// The mana pool is a place on the shelf, not a badge that comes and goes.
    ///
    /// Three claims in one, and the first is the one that changed. The chip
    /// this replaces was drawn only "while the seat has something to answer",
    /// so at every opponent's priority the label blinked out — movement in
    /// the corner of the eye carrying no information at all. AX §4.1 keeps it
    /// standing, because a reserved column costs nothing to leave occupied
    /// where a floating box cost the table a piece of itself.
    ///
    /// Then: an empty pool is an em dash and not six zeroes, and mana in it
    /// is a numeral beside a disc rather than a row of discs to count.
    #[test]
    fn the_mana_pool_is_a_place_and_not_a_badge() {
        let label = Phrase::ManaPool.text(Lang::En).to_string();
        let dash = "\u{2014}".to_string();

        let mut watching = bar_of(duel_watching());
        let lines = said(&mut watching);
        assert!(
            lines.contains(&label),
            "the pool keeps its place while this seat waits: {lines:?}"
        );
        assert!(
            lines.contains(&dash),
            "nothing floating is one fact, written once: {lines:?}"
        );

        // And with mana in it the dash is gone and the count is a numeral.
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
        let lines = said(&mut floating);
        assert!(
            lines.contains(&"\u{00d7}3".to_string()) && lines.contains(&"\u{00d7}1".to_string()),
            "three green and one restricted white, as numerals: {lines:?}"
        );
        assert!(
            !lines.contains(&dash),
            "the dash stands *instead of* the entries, not beside them: \
             {lines:?}"
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
        let pooled = |green: u16, prompt: &str| {
            let mut duel = duel_watching();
            duel.last_error = Some(prompt.to_string());
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
        };
        let entries = |app: &mut App| {
            let mut q = app.world_mut().query::<(Entity, &ledge::pool::PoolEntry)>();
            q.iter(app.world()).map(|(e, _)| e).collect::<Vec<_>>()
        };

        let mut app = bar_of(pooled(1, "one"));
        let first = entries(&mut app);
        assert_eq!(first.len(), 1, "one colour is floating, so one entry");
        let before = said(&mut app).join("|");

        *app.world_mut().resource_mut::<Duel>() = pooled(2, "two");
        app.update();
        assert_eq!(
            entries(&mut app),
            first,
            "the same mana, more of it: the entry is written, not replaced"
        );
        assert_ne!(
            said(&mut app).join("|"),
            before,
            "this test's premise is that the shelf was rebuilt under it"
        );

        // Spent. With motion off the fade is over on the frame it starts.
        *app.world_mut().resource_mut::<Duel>() = pooled(0, "three");
        app.update();
        assert!(
            entries(&mut app).is_empty(),
            "a spent mana leaves, rather than being left behind"
        );
        // And the em dash waits for it: one more frame, because the row can
        // only say "nothing floating" once nothing is on it saying otherwise.
        app.update();
        assert!(
            said(&mut app).contains(&"\u{2014}".to_string()),
            "the empty pool says so again once the last pip has gone"
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
}
