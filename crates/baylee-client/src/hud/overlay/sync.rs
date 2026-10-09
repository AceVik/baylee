//! The rebuild: `sync_overlay`.

#[allow(clippy::wildcard_imports)] // the overlay's shared vocabulary
use super::*;

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
    // With the font's own asset, which the text faces are fitted by: in one
    // parameter, because the system is at its limit of sixteen.
    (fonts, font_assets): (Res<UiFonts>, Option<Res<Assets<Font>>>),
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
        widths: crate::face::Widths::of(font_assets.as_deref().and_then(|a| a.get(&fonts.text)))
            .at(mode.step),
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
    let prompt = duel.headline(lang, &texts);
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
    let focus = duel.visiting;
    let preview_scale = settings.preview_scale;
    let armed_deed = duel.armed.clone();
    // Rounded to whole pixels: a window being dragged reports fractional
    // sizes, and a revision keyed on an `f32` would rebuild the whole tree on
    // a sub-pixel wobble.
    let canvas = windows
        .single()
        .map_or((1200, 800), |w| (w.width() as i32, w.height() as i32));
    let number = duel
        .interaction
        .as_ref()
        .and_then(|i| i.edits_number().then(|| i.number()));
    let choice = duel
        .interaction
        .as_ref()
        .and_then(baylee_client_core::Interaction::chosen_index);

    let top = duel
        .board
        .as_ref()
        .and_then(|board| board.stack.first())
        .map(|item| item.id);
    let (stack_scroll, full_height) = tree.stack_scroll.iter().next().map_or_else(
        || (ScrollPosition::default(), super::stack::STACK_FULL_HEIGHT),
        |(scroll, node, body)| (scroll.clone(), body.full_height(node, top)),
    );
    let stack_window = super::stack::window_start(stack_scroll.y, full_height);
    if revision.seq == seq
        && revision.lang == Some(lang)
        && revision.stack_selected == duel.stack_selected
        && revision.ability_orders == prefs.all().ability_orders
        && revision.stack_window == stack_window
        && revision.hand_order == duel.hand_order
        && revision.prompt == prompt
        && revision.error == error
        && revision.link_note == link_note
        && revision.hovered == hovered
        && revision.hovered_log == duel.hovered_log
        && revision.selected == selected
        && revision.orders.as_ref().is_some_and(|o| o.same_as(&orders))
        && revision.autopilot == autopilot
        && revision.focus == focus
        && (revision.preview_scale - preview_scale).abs() < f32::EPSILON
        && revision.face_step == mode.step
        && revision.faces == faces.always()
        && revision.texts == texts.generation()
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
    revision.lang = Some(lang);
    revision.stack_selected = duel.stack_selected;
    revision
        .ability_orders
        .clone_from(&prefs.all().ability_orders);
    revision.stack_window = stack_window;
    revision.hand_order = duel.hand_order;
    revision.prompt.clone_from(&prompt);
    revision.error.clone_from(&error);
    revision.link_note = link_note;
    revision.hovered = hovered;
    revision.hovered_log = duel.hovered_log;
    revision.selected.clone_from(&selected);
    revision.orders = Some(orders);
    revision.autopilot = autopilot;
    revision.focus = focus;
    revision.preview_scale = preview_scale;
    revision.face_step = mode.step;
    revision.faces = faces.always();
    revision.texts = texts.generation();
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
                && !tree.players.contains(*child)
                && !tree.menu.contains(*child)
                && !tree.log.contains(*child)
                && !tree.ai_log.contains(*child)
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
        // The players' strip (#264), at the shelf's rung and spawned before
        // the drawer so that the drawer, growing out of the same edge,
        // stands over it: a question is read over the roster. See
        // [`ledge::players`].
        let players = ledge::players::spawn_players_strip(&mut commands);
        commands.entity(root).add_child(players);
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
        // And the game log's panel (#262), for the menu's reason and one of
        // its own: its lines are appended as they arrive, and a panel swept
        // with the tree would write every line again on each pointer move.
        let log = ledge::log::spawn_log_panel(&mut commands);
        // The AI log is a debug build's alone (`docs/protocol.md` §"An AI
        // seat's reasoning"): a release client stands no panel for it.
        if cfg!(debug_assertions) {
            let ai_log = ledge::ai_log::spawn(&mut commands);
            commands.entity(root).add_child(ai_log);
        }
        commands.entity(root).add_child(log);
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
        //
        // Or over a card a log line names, while the pointer is on its link
        // (#300): the printing the line showed, beside the pointer, and in
        // place of any card the keyboard cursor holds. Everything below that
        // reads the table's own object (its strip, its count, its sheen, the
        // stack's sentence) is then asked about none, since the line's card
        // is not the object as it stands now and may be nowhere at all.
        let hovered = hovered.filter(|_| duel.hovered_log.is_none());
        let anchor = match duel.hovered_log {
            Some(log) => log
                .link
                .art(ArtSize::Normal)
                .map(|art| (Some(art), PreviewAt::Pointer(log.at))),
            None => preview_anchor(
                board,
                view,
                hovered,
                layout,
                duel.hand_scroll,
                duel.hovered_at,
            ),
        };
        if let Some((art, anchor)) = anchor {
            let scale = settings.preview_scale;
            let window = windows.single().map_or(Vec2::new(1200.0, 800.0), |w| {
                Vec2::new(w.width(), w.height())
            });
            // What the scale slider asked for, and then what this window can
            // actually show: a preview larger than the screen is cut off
            // wherever it is placed, and no amount of arithmetic in
            // `preview_place` can rescue it. The slider is a preference; the
            // window is not.
            let want = super::hand::preview_want(scale);
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
            let art_size = preview_with_footer_size(want, window, asked);
            let (img_w, img_h) = (art_size.x, art_size.y);
            let slip_h = slip
                .as_ref()
                .map_or(0.0, |s| super::slip::height(&runs, img_w, s.kind.is_some()));
            // The panel is the picture plus its six pixels of padding on
            // every side, and the sheet under it when there is one.
            let panel =
                art_size + Vec2::splat(12.0) + Vec2::new(0.0, slip_h + super::preview_keys::HEIGHT);
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
            // And the same count: the ×12 on the table is the one thing the
            // art under it cannot say. A badge over the card's top-right
            // corner, as it stands over it on a duel's felt (#261), so the
            // panel is placed with the badge's reach above it — a preview at
            // the window's top edge would stand its count off the screen —
            // and the bubble's clip lets it out as far.
            let count = baylee_client_core::cardplate::count_word(
                hovered
                    .and_then(|id| board.group(id))
                    .map_or(1, baylee_client_core::board::CardGroup::count),
            );
            let badge = if count == 0 {
                None
            } else {
                surfaces.badge(count)
            };
            let reach = if badge.is_some() {
                badge_reach(img_w)
            } else {
                0.0
            };
            // And the plate, where the print cannot say the numbers (the
            // owner, 25.09): beside the printed box over the black border,
            // so a little of it hangs past the card's right edge. The panel
            // is placed as one that much wider, and the clip lets it out.
            let plate = hovered
                .and_then(|id| view.object(id))
                .and_then(crate::platemat::words_of);
            let plate_over = if plate.is_some() {
                plate_reach(img_w)
            } else {
                0.0
            };
            // And the shells it wears on the felt (the PM, 25.09), from the
            // door the table asks: a dome's foot off its sides and its foot,
            // a wall over its top. The panel is placed as one that much
            // bigger all round, and the clip lets them out as far.
            let shells = hovered
                .and_then(|id| board.group(id))
                .map(crate::shellmat::Shells::of)
                .unwrap_or_default();
            let shell = shell_reach(img_w, shells);
            let [shell_side, shell_top] = shell;
            let place = place_around(anchor, panel, window, keep_out, [reach, plate_over], shell);
            let [under, over] = shells.standing();
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
                        Some(shown) => {
                            CardLook::art(shown, finish_of(statics, Some(shown))).with_sweep(sweep)
                        }
                        None => CardLook::back(FinishTreatment::Plain).with_sweep(sweep),
                    }
                },
                cards.as_mut(),
                &faces.widths,
            );
            // The strip, lying on the art where it lies on the table (#274,
            // #298): an object of its own over the card, at the same place in
            // card widths, saying what it says there. The preview is the
            // same permanent drawn larger, so it says the same numbers, on
            // its plate and in its chip: a 2/2 under an anthem is a 3/3 on
            // the table, and a preview showing the printed 2/2 would put two
            // answers for one creature on one screen. A card in hand has no view object and so no strip,
            // which is right — its printed body is what it is. Over the art
            // only: a card showing its text face says it in words.
            let strip = hovered
                .and_then(|id| view.object(id))
                .map(crate::marksmat::strip_of)
                .filter(|strip| !strip.is_empty());
            if let Some(strip) = strip
                && built.is_none()
                && let Some(material) = surfaces.strip(strip)
            {
                let [x0, y0, x1, y1] = baylee_client_core::cardrail::quad_rect();
                let down = img_h / baylee_client_core::cardrail::CARD_TALL;
                let node = commands
                    .spawn((
                        MaterialNode(material),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(x0 * img_w),
                            top: px(y0 * down),
                            width: px((x1 - x0) * img_w),
                            height: px((y1 - y0) * down),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(visual).add_child(node);
            }
            let tooltip = commands
                .spawn((
                    PreviewBounds,
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(place.y),
                        left: px(place.x),
                        padding: UiRect::all(px(PREVIEW_PAD)),
                        // A column, because the bubble is a card and — when
                        // the pointer is on the stack — a sheet under it.
                        flex_direction: FlexDirection::Column,
                        border_radius: preview_radius(img_w),
                        overflow: Overflow::clip(),
                        overflow_clip_margin: OverflowClipMargin::padding_box()
                            .with_margin(reach.max(plate_over).max(shell_side).max(shell_top)),
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
                            bottom: px(4.0 + slip_h + super::preview_keys::HEIGHT),
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
                    CardLook::art(back, finish_of(statics, Some(back))),
                ),
                // No corner on the back: power, toughness and counters are
                // printed on the face and a card lying face down shows none
                // of them.
                None => (textures.card_back(), CardLook::back(FinishTreatment::Plain)),
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
                &faces.widths,
            );
            commands.entity(far).insert((
                crate::flip::Side::Back,
                // Hidden until the turn passes the quarter, where the card is
                // edge-on and the swap cannot be seen.
                Visibility::Hidden,
                face_node(img_w, img_h),
            ));
            commands.entity(frame).add_child(far);
            // The shells under the card's own objects, the wall and the rim,
            // hang off the frame for the badge's reason below, and are laid
            // before them: on the table they lie under the strip, the plate
            // and the badge too.
            for look in under {
                spawn_shell(&mut commands, &mut surfaces, frame, look, img_w, img_h);
            }
            // The badge hangs off the frame and not off the face, whose node
            // clips to the card: it is one side's, so it turns with the front
            // and is hidden with it at the quarter turn.
            if let Some(material) = badge {
                let [x0, y0, x1, y1] =
                    baylee_client_core::cardplate::badge_quad_rect(crate::badgemat::PREVIEW);
                let down = img_h / baylee_client_core::cardrail::CARD_TALL;
                let node = commands
                    .spawn((
                        MaterialNode(material),
                        crate::flip::Side::Front,
                        Visibility::Inherited,
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(x0 * img_w),
                            top: px(y0 * down),
                            width: px((x1 - x0) * img_w),
                            height: px((y1 - y0) * down),
                            ..default()
                        },
                    ))
                    .id();
                commands.entity(frame).add_child(node);
            }
            // The plate hangs off the frame for the badge's reason, and over
            // the art only, for the strip's: a text face says it in words.
            if let Some(words) = plate
                && built.is_none()
                && let Some(material) = surfaces.plate(words)
            {
                let kind = words.word >> baylee_client_core::cardplate::KIND_SHIFT;
                let [x0, y0, x1, y1] = crate::platemat::preview_quad(kind);
                let down = img_h / baylee_client_core::cardrail::CARD_TALL;
                let node = commands
                    .spawn((
                        MaterialNode(material),
                        crate::flip::Side::Front,
                        Visibility::Inherited,
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(x0 * img_w),
                            top: px(y0 * down),
                            width: px((x1 - x0) * img_w),
                            height: px((y1 - y0) * down),
                            ..default()
                        },
                    ))
                    .id();
                commands.entity(frame).add_child(node);
            }
            // A dome last, over everything the card says: it is glass over
            // the whole card, as on the table.
            for look in over {
                spawn_shell(&mut commands, &mut surfaces, frame, look, img_w, img_h);
            }
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
            let legend = super::preview_keys::spawn(&mut commands, &fonts, lang, img_w);
            commands.entity(tooltip).add_child(legend);
            commands.entity(root).add_child(tooltip);
            // Over the end screen once it stands: its log's links open this
            // preview, and one drawn at the overlay's own rung is behind the
            // sheet the pointer is on. Everything below that hangs off the
            // preview stands with it.
            let over_the_finish = duel.ending().is_some();
            let lift = |commands: &mut Commands, node: Entity| {
                if over_the_finish {
                    commands
                        .entity(node)
                        .insert(GlobalZIndex(G_PREVIEW_OVER_FINISH));
                }
            };
            lift(&mut commands, tooltip);

            // What the preview stands in, with the card underneath a copy
            // once it stands beside it: the cards attached to it go beside
            // both.
            let mut taken = preview_taken(place, panel, plate_over, shell);

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
                let size = Vec2::new(thumb_w, thumb_h + CAPTION_H);
                let at = underneath_place(taken, size, window);
                taken = taken.union(Rect::from_corners(at, at + size));
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
                    CardLook::art(under, finish_of(statics, Some(under))),
                    cards.as_mut(),
                    &faces.widths,
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
                lift(&mut commands, beside);
            }

            // ---- the cards attached to it (#305) -----------------------
            //
            // On the table they lie under their host and peek out past it,
            // or, where the row leaves them no room to peek out whole, lie
            // flush under it and the host wears the attachment mark, which
            // says only how many. This says which: each as a small card, in
            // columns of two, captioned as the card underneath a copy is.
            // Each drawn as the preview draws a card: its art, or where it
            // has none (a Role is a token) its characteristics.
            let attached = hovered
                .and_then(|id| board.group(id))
                .map_or(&[][..], |group| group.attached.as_slice());
            if !attached.is_empty() {
                const GAP: f32 = 4.0;
                let thumb_w = (img_w * 0.34).max(56.0);
                let thumb_h = thumb_w * 88.0 / 63.0;
                let rows = attached.len().min(2);
                let columns = attached.len().div_ceil(2);
                #[expect(clippy::cast_precision_loss)] // a handful of cards
                let size = Vec2::new(
                    columns as f32 * (thumb_w + GAP) - GAP,
                    rows as f32 * (thumb_h + GAP) - GAP + CAPTION_H,
                );
                let at = underneath_place(taken, size, window);
                let grid = commands
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(GAP),
                        ..default()
                    })
                    .id();
                for pair in attached.chunks(2) {
                    let column = commands
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(GAP),
                            ..default()
                        })
                        .id();
                    for group in pair {
                        let key = group.art;
                        let built = match key {
                            Some(_) => None,
                            None => {
                                preview_face(&faces, view, &textures, group.representative, None)
                            }
                        };
                        let image = match key {
                            Some(key) => textures.get(key, statics, &assets),
                            None => textures.card_back(),
                        };
                        let card = spawn_card_art(
                            &mut commands,
                            lang,
                            image,
                            built.as_ref(),
                            thumb_w,
                            thumb_h,
                            crate::face::Detail::Compact,
                            &fonts,
                            match key {
                                Some(key) => CardLook::art(key, finish_of(statics, Some(key))),
                                None => CardLook::back(FinishTreatment::Plain),
                            },
                            cards.as_mut(),
                            &faces.widths,
                        );
                        commands.entity(card).insert(upward_shadow());
                        commands.entity(column).add_child(card);
                    }
                    commands.entity(grid).add_child(column);
                }
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
                        PreviewAttached,
                        children![(
                            Text::new(Phrase::CardAttached.text(lang).to_string()),
                            tf(&fonts, 9.0),
                            TextColor(palette::MUTED),
                        )],
                    ))
                    .id();
                commands.entity(beside).add_child(grid);
                // Beside the preview is over the permanents next to the
                // hovered one, as the card underneath a copy is.
                commands
                    .entity(beside)
                    .insert_recursive::<Children>(Pickable::IGNORE);
                commands.entity(root).add_child(beside);
                lift(&mut commands, beside);
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
                lift(&mut commands, tail);
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
        // The window is in the revision, so one crossing a phone's height
        // rebuilds the sentence's box at its other height.
        #[allow(clippy::cast_precision_loss)] // a window's size in pixels
        let window = Vec2::new(canvas.0 as f32, canvas.1 as f32);
        let text_lines = super::stack::text_lines(window.y);
        // Where it stands, and on a phone its compact shape; the drawer's
        // part `fold_the_stack` keeps up to date every frame.
        let room = super::stack::panel_room(
            window,
            duel.hand_shown,
            super::stack::strip_right(tree.players_strip.iter()),
        );
        let stack = spawn_stack_panel(
            &mut commands,
            duel.stack_selected,
            &prefs.all().ability_orders,
            stack_scroll,
            (full_height, text_lines, room),
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
