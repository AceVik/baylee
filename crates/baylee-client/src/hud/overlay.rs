//! The retained HUD tree: the seat tabs, the own-board overlay, and the
//! one system that rebuilds all of it.
//!
//! Rebuilt only when [`HudRevision`] says something it draws has changed —
//! a rebuild per frame would cost more than the whole table does.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::interaction::Prompt;

/// How much of the own-board overlay stands above the hand bar when it is
/// closed — which is the knob, and nothing else.
///
/// One constant because it is one fact stated in three places: the knob's own
/// height, and the closed `top` computed both where the panel is spawned and
/// where it is animated. Written out three times, it was possible for the
/// panel to be taller than the handle it is supposed to be showing, and it
/// was: the tops of your own permanents stood above the hand bar, clipped to
/// their title bars, looking exactly like cards left behind by the cards you
/// had played.
pub(crate) const KNOB_H: f32 = 14.0;

/// The closed panel's `top`, given the window's height.
///
/// Paired with `bottom: HAND_BAR_H`, this makes the closed panel exactly
/// [`KNOB_H`] tall — which is what the lanes container has to clip against.
#[must_use]
pub(crate) fn closed_overlay_top(window_h: f32) -> f32 {
    window_h - HAND_BAR_H - KNOB_H
}

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
    ui_materials: Option<ResMut<UiCardMaterials>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    *revision = HudRevision::default();
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
    existing: Query<Entity, With<HudRoot>>,
    mut textures: ResMut<CardTextures>,
    assets: Res<AssetServer>,
    windows: Query<&Window>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    prefs: Res<crate::prefs::Prefs>,
    texts: Res<crate::cardtext::CardTexts>,
    mode: Res<crate::face::FaceMode>,
    // Both come from the render plugins. A headless app has neither, and
    // every card below falls back to a plain image rather than growing a
    // second code path for it.
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
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
    };
    let lang = Lang::of(&settings.lang);
    let seq = duel.board.as_ref().map(|b| b.seq);
    // A finished game says who won, not merely that it is finished — and a
    // team game says which team, which is why this is answered here and not
    // in the prompt: the seat's own team lives in the roster.
    let ending = duel.interaction.as_ref().and_then(|i| {
        let baylee_engine::choice::Pending::GameOver(result) = i.pending() else {
            return None;
        };
        let statics = duel.statics.as_ref()?;
        let seat = statics.your_seat;
        let team = statics
            .seats
            .iter()
            .find(|s| s.player == seat)
            .and_then(|s| s.team);
        Some(baylee_client_core::interaction::verdict(
            lang, result, seat, team,
        ))
    });
    let prompt = ending
        .or_else(|| duel.interaction.as_ref().map(|i| i.prompt().headline(lang)))
        .or_else(|| duel.last_error.clone());
    let hovered = duel.hovered;
    let selected: Vec<ObjectId> = duel
        .interaction
        .as_ref()
        .map(|i| i.selected().to_vec())
        .unwrap_or_default();
    let orders = prefs.orders().clone();
    let autopilot = duel.autopilot;
    let combat = duel.interaction.as_ref().and_then(|i| {
        i.focus_position()
            .map(|(focus, count)| (focus, count, i.declared()))
    });
    let ability_menu = duel.ability_menu;
    let ability_pick = duel.ability_pick;
    let focus = duel.focus;
    let overlay_open = duel.overlay_open;
    let preview_scale = settings.preview_scale;
    let browser = (
        duel.browser.is_open(),
        duel.browser.tab(),
        duel.browser.filter().to_string(),
    );
    let menu = (duel.can_offer_draw(), duel.concede_armed);
    let armed_deed = duel.armed.clone();
    let number = duel
        .interaction
        .as_ref()
        .and_then(|i| matches!(i.prompt(), Prompt::ChooseNumber { .. }).then(|| i.number()));

    if revision.seq == seq
        && revision.prompt == prompt
        && revision.hovered == hovered
        && revision.selected == selected
        && revision.orders.as_ref().is_some_and(|o| o.same_as(&orders))
        && revision.autopilot == autopilot
        && revision.focus == focus
        && revision.overlay_open == overlay_open
        && (revision.preview_scale - preview_scale).abs() < f32::EPSILON
        && revision.faces == faces.always()
        && revision.texts == texts.len()
        && revision.combat == combat
        && revision.ability_menu == ability_menu
        && revision.ability_pick == ability_pick
        && revision.browser == browser
        && revision.menu == menu
        && revision.armed == armed_deed
        && revision.number == number
        && !existing.is_empty()
    {
        return;
    }
    revision.seq = seq;
    revision.prompt.clone_from(&prompt);
    revision.hovered = hovered;
    revision.selected.clone_from(&selected);
    revision.orders = Some(orders.clone());
    revision.autopilot = autopilot;
    revision.focus = focus;
    revision.overlay_open = overlay_open;
    revision.preview_scale = preview_scale;
    revision.faces = faces.always();
    revision.texts = texts.len();
    revision.combat = combat;
    revision.ability_menu = ability_menu;
    revision.ability_pick = ability_pick;
    revision.browser = browser;
    revision.menu = menu;
    revision.armed.clone_from(&armed_deed);
    revision.number = number;

    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let (Some(board), Some(view)) = (duel.board.as_ref(), duel.view.as_ref()) else {
        return;
    };

    // Which cards this choice will actually accept. `selected` says what a
    // player has picked; this says what they *may* pick, which is the thing
    // the hand had no way of showing: a cleanup discard lit nothing up at
    // all, so the only clue that the hand was clickable was clicking it.
    let selectable: Vec<ObjectId> = duel
        .interaction
        .as_ref()
        .map(|i| {
            board
                .hand
                .iter()
                .map(|c| c.id)
                .filter(|id| i.is_selectable(*id))
                .collect()
        })
        .unwrap_or_default();

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

    // ---- top: the full-width tab bar — ALL players left, menu right ----
    let tabs = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(8), px(6)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Pickable::IGNORE,
        ))
        .id();
    let players_row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for seat in &view.seats {
        let tab = spawn_player_tab(
            &mut commands,
            lang,
            view,
            duel.statics.as_ref(),
            seat,
            focus,
            &fonts,
        );
        commands.entity(players_row).add_child(tab);
    }
    commands.entity(tabs).add_child(players_row);

    let menu_row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    // A draw needs this seat's own priority (CR 104.4a, and `offer_draw`
    // refuses anything else), so the button says so instead of being a live
    // button whose usual answer is an error in the prompt bar. Concede is
    // always legal and is greyed by nothing — what it has instead is a second
    // press, because there is no undo behind it.
    if duel.priority_held() {
        spawn_hold(&mut commands, &fonts, lang, menu_row);
    }
    let armed = duel.concede_armed;
    for (action, label, enabled) in [
        (
            MenuAction::OfferDraw,
            Phrase::OfferADraw.text(lang),
            duel.can_offer_draw(),
        ),
        (
            MenuAction::Concede,
            if armed {
                Phrase::ConcedeConfirm.text(lang)
            } else {
                Phrase::Concede.text(lang)
            },
            true,
        ),
    ] {
        let lit = match (action, armed) {
            (MenuAction::Concede, true) => palette::DANGER,
            _ if enabled => palette::PANEL_LIT,
            _ => palette::PANEL,
        };
        let ink = match (action, armed) {
            (MenuAction::Concede, true) => palette::PANEL,
            _ if enabled => palette::INK,
            _ => palette::DEAD,
        };
        let button = commands
            .spawn((
                MenuButton { action },
                Node {
                    padding: UiRect::axes(px(12), px(6)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(lit),
                soft_shadow(),
                children![(Text::new(label), tf(&fonts, 13.0), TextColor(ink))],
            ))
            .id();
        commands.entity(menu_row).add_child(button);
    }
    commands.entity(tabs).add_child(menu_row);
    commands.entity(root).add_child(tabs);

    // ---- right: the phase rail (opponents' phases top, yours bottom) ---
    let window_h = windows.single().map_or(800.0, Window::height);
    let rail = spawn_phase_rail(
        &mut commands,
        lang,
        view,
        &orders,
        autopilot,
        &fonts,
        window_h,
        duel.statics.as_ref(),
    );
    commands.entity(root).add_child(rail);

    // ---- prompt bar (choice headline + answer buttons), above the hand,
    // padded clear of the phase rail ---------------------------------------
    if let Some(text) = prompt {
        let waiting = !duel.is_my_turn_to_act();
        let bar = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(HAND_BAR_H + 10.0),
                    right: px(RAIL_W + 12.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    padding: UiRect::axes(px(14), px(8)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(palette::PANEL),
                soft_shadow(),
            ))
            .id();
        let headline = commands
            .spawn((
                Text::new(text),
                tf(&fonts, 18.0),
                TextColor(if waiting {
                    palette::MUTED
                } else {
                    palette::ACCENT
                }),
            ))
            .id();
        commands.entity(bar).add_child(headline);

        // A choice that is answered by clicking has to say so. The prompt
        // bar used to draw "Discard 1 card(s)" and stop: no button, because
        // nothing is submittable until something is picked, and no hint,
        // because none existed. A player who did not already know to click
        // their hand had no way to find out.
        if let Some(hint) = duel
            .interaction
            .as_ref()
            .filter(|i| !waiting && i.selected().is_empty())
            .and_then(|i| pick_hint(&i.prompt()))
        {
            let line = commands
                .spawn((
                    Text::new(hint.text(lang).to_string()),
                    tf(&fonts, 12.0),
                    TextColor(palette::MUTED),
                ))
                .id();
            commands.entity(bar).add_child(line);
        }

        // ---- combat: what the next declaration is aimed at -----------------
        //
        // Combat is the one choice where clicking a creature is not enough:
        // the engine asks *which* defender, and a player who cannot see the
        // answer is guessing. The line says where the aim points and how many
        // declarations stand, and it is the same aim the keyboard cycles.
        if let Some(line) = duel
            .interaction
            .as_ref()
            .filter(|i| i.is_combat() && !waiting)
            .and_then(|i| combat_line(i, view, duel.statics.as_ref(), lang))
        {
            let aim = commands
                .spawn((Text::new(line), tf(&fonts, 13.0), TextColor(palette::MUTED)))
                .id();
            commands.entity(bar).add_child(aim);
        }

        // ---- the number stepper -------------------------------------------
        //
        // The one choice with nothing on the table to click. The headline
        // already says the range; what was missing was the value itself and
        // any way at all to change it with a pointer.
        if let (Some(value), false) = (number, waiting) {
            let row = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(10),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            let minus = spawn_step(&mut commands, &fonts, -1, "\u{2212}");
            let shown = commands
                .spawn((
                    Text::new(value.to_string()),
                    tf(&fonts, 20.0),
                    TextColor(palette::INK),
                ))
                .id();
            let plus = spawn_step(&mut commands, &fonts, 1, "+");
            commands.entity(row).add_children(&[minus, shown, plus]);
            commands.entity(bar).add_child(row);
        }

        // Answer buttons, matching the pending choice.
        let combat_answers = [
            (PromptAction::AimNext, Phrase::AimNext.text(lang)),
            (PromptAction::Confirm, Phrase::Attack.text(lang)),
            (PromptAction::DeclareNothing, Phrase::DeclareNone.text(lang)),
        ];
        let block_answers = [
            (PromptAction::AimNext, Phrase::AimNext.text(lang)),
            (PromptAction::Confirm, Phrase::Block.text(lang)),
            (PromptAction::DeclareNothing, Phrase::DeclareNone.text(lang)),
        ];
        let mulligan_answers = [
            (PromptAction::Keep, Phrase::KeepHand.text(lang)),
            (PromptAction::Mulligan, Phrase::TakeMulligan.text(lang)),
        ];
        let yes_no_answers = [
            (PromptAction::Yes, Phrase::ActAnswerYes.text(lang)),
            (PromptAction::No, Phrase::ActAnswerNo.text(lang)),
        ];
        let ok_answer = [(PromptAction::Confirm, Phrase::ConfirmOk.text(lang))];
        let answers: &[(PromptAction, &str)] = if waiting {
            &[]
        } else {
            match duel
                .interaction
                .as_ref()
                .map(baylee_client_core::Interaction::pending)
            {
                Some(baylee_engine::choice::Pending::Mulligan { .. }) => &mulligan_answers,
                Some(baylee_engine::choice::Pending::YesNo { .. }) => &yes_no_answers,
                // Combat always offers all three, including with nothing
                // declared: "none" is a real answer, and the step does not
                // end until somebody gives one.
                Some(baylee_engine::choice::Pending::ChooseAttackers { .. }) => &combat_answers,
                Some(baylee_engine::choice::Pending::ChooseBlockers { .. }) => &block_answers,
                Some(_)
                    if duel
                        .interaction
                        .as_ref()
                        .is_some_and(baylee_client_core::Interaction::can_confirm) =>
                {
                    &ok_answer
                }
                _ => &[],
            }
        };
        if !answers.is_empty() {
            let row = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(6),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            for (action, label) in answers {
                let button = commands
                    .spawn((
                        PromptButton { action: *action },
                        Node {
                            padding: UiRect::axes(px(12), px(5)),
                            border_radius: btn_radius(),
                            ..default()
                        },
                        BackgroundColor(palette::ACCENT),
                        soft_shadow(),
                        children![(
                            Text::new(*label),
                            tf(&fonts, 13.0),
                            TextColor(palette::PANEL),
                        )],
                    ))
                    .id();
                commands.entity(row).add_child(button);
            }
            commands.entity(bar).add_child(row);
        }

        // ---- the type-to-filter box, for the one choice whose list is too
        // long to look at. Drawn whether or not anything matches: a filter
        // with no rows under it is exactly when a player needs to see what
        // they typed.
        if !waiting
            && let Some(Prompt::ChooseSubtype { .. }) = duel
                .interaction
                .as_ref()
                .map(baylee_client_core::Interaction::prompt)
        {
            let field = commands
                .spawn((
                    Node {
                        padding: UiRect::axes(px(10), px(5)),
                        border: UiRect::all(px(1)),
                        border_radius: btn_radius(),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(palette::PANEL),
                    BorderColor::all(palette::ACTIVE),
                    Pickable::IGNORE,
                ))
                .id();
            // A caret with nothing before it, so an empty box still reads as
            // somewhere to type rather than as a blank panel.
            let text = commands
                .spawn((
                    Text::new(format!("{}_", duel.subtype_filter)),
                    tf(&fonts, 14.0),
                    TextColor(palette::INK),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(field).add_child(text);
            commands.entity(bar).add_child(field);
        }

        // ---- the indexed chooser: a colour, a seat, a way to cast ---------
        //
        // Its own row, above the ability menu and below the answers, because
        // it is neither: these are not "OK" and they are not things to do
        // while holding priority — they are *the* answer, and picking one
        // sends it. Until this existed a tapped dual land drew "Choose a
        // colour" with nothing under it and the game stopped there.
        if let Some(rows) = duel
            .interaction
            .as_ref()
            .filter(|_| !waiting)
            .map(baylee_client_core::Interaction::prompt)
            .and_then(|p| {
                crate::choices::options(&p, lang, duel.statics.as_ref(), &duel.subtype_filter)
            })
            .filter(|rows| !rows.is_empty())
        {
            let picked = duel
                .interaction
                .as_ref()
                .and_then(baylee_client_core::Interaction::chosen_index);
            let row = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(6),
                        flex_wrap: FlexWrap::Wrap,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            for option in &rows {
                let on = picked == Some(option.index);
                let button = commands
                    .spawn((
                        ChoiceButton {
                            index: option.index,
                        },
                        Node {
                            padding: UiRect::axes(px(10), px(5)),
                            border: UiRect::all(px(1)),
                            border_radius: btn_radius(),
                            column_gap: px(5),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(if on {
                            palette::ACTIVE
                        } else {
                            palette::PANEL_LIT
                        }),
                        BorderColor::all(if on { palette::ACTIVE } else { palette::MUTED }),
                        soft_shadow(),
                    ))
                    .id();
                if let Some(pip) = option.pip {
                    let mark = crate::manaui::spawn_pip(&mut commands, &fonts, pip, 20.0);
                    commands.entity(button).add_child(mark);
                }
                if !option.label.is_empty() {
                    let text = commands
                        .spawn((
                            Text::new(option.label.clone()),
                            tf(&fonts, 13.0),
                            TextColor(if on { palette::PANEL } else { palette::INK }),
                            Pickable::IGNORE,
                        ))
                        .id();
                    commands.entity(button).add_child(text);
                }
                if let Some(cost) = option.cost {
                    for pip in baylee_client_core::manapip::cost(&cost) {
                        let mark = crate::manaui::spawn_pip(&mut commands, &fonts, pip, 15.0);
                        commands.entity(button).add_child(mark);
                    }
                }
                commands.entity(row).add_child(button);
            }
            commands.entity(bar).add_child(row);
        }

        // What is armed, and the way back out of it. Its own row, above the
        // chooser it replaces: arming is where the chooser ends, and the two
        // are never open at once.
        if let Some(label) = duel
            .armed
            .as_ref()
            .and_then(|a| armed_label(&duel, lang, a))
        {
            let row = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(6),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            spawn_armed(&mut commands, &fonts, lang, row, &label);
            commands.entity(bar).add_child(row);
        }

        // The ability chooser, when a permanent was clicked that offers more
        // than one thing. Its own row rather than more entries in `answers`,
        // because these are not answers to the pending choice — they are
        // things to *do* while holding priority, and mixing them with "OK"
        // would put a mana ability next to the button that ends the turn.
        if let Some(options) = duel
            .ability_menu
            .and_then(|object| ability_options(&duel, lang, object))
            .filter(|options| options.len() > 1)
        {
            let row = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(6),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            for (index, option) in options.iter().enumerate() {
                // The keyboard's entry is drawn as the chosen one, so the two
                // ways of answering the menu are visibly the same menu.
                let picked = index == duel.ability_pick;
                let button = commands
                    .spawn((
                        AbilityButton { index },
                        Node {
                            padding: UiRect::axes(px(12), px(5)),
                            border: UiRect::all(px(1)),
                            border_radius: btn_radius(),
                            ..default()
                        },
                        BackgroundColor(if picked {
                            palette::ACTIVE
                        } else {
                            palette::PANEL_LIT
                        }),
                        BorderColor::all(if picked {
                            palette::ACTIVE
                        } else {
                            palette::MUTED
                        }),
                        soft_shadow(),
                        children![(
                            Text::new(option.label.clone()),
                            tf(&fonts, 13.0),
                            TextColor(if picked { palette::PANEL } else { palette::INK }),
                        )],
                    ))
                    .id();
                commands.entity(row).add_child(button);
            }
            commands.entity(bar).add_child(row);
        }
        commands.entity(root).add_child(bar);
    }

    // ---- bottom: the hand bar (always on top) + commander zone ----------
    if let Some(statics) = duel.statics.as_ref() {
        let commanders = view
            .command
            .get(view.seat.get() as usize)
            .map_or(&[][..], Vec::as_slice);
        let cmdr_width = if commanders.is_empty() { 0.0 } else { 110.0 };
        let available = windows
            .single()
            .map_or(1200.0, |w| (w.width() - 20.0 - cmdr_width).max(0.0));
        let layout = hand_layout(board.hand.len(), HAND_CARD_W, available);
        let hand_bar = spawn_hand_bar(
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
            cards.as_mut(),
        );
        commands.entity(root).add_child(hand_bar);

        // ---- card preview: a speech-bubble tooltip over the hovered
        // card (hand, own battlefield, or command zone). No title text —
        // the image is big enough to read.
        if let Some((art, anchor)) = preview_anchor(board, view, hovered, layout, duel.hand_scroll)
        {
            let scale = settings.preview_scale.clamp(0.5, 1.75);
            let img_w = 308.0 * scale;
            let img_h = img_w * 88.0 / 63.0;
            let panel_w = img_w + 12.0;
            let win_w = windows.single().map_or(1200.0, Window::width);
            let anchor = anchor.unwrap_or(win_w / 2.0);
            // Always fully in the viewport.
            let left = (anchor - panel_w / 2.0).clamp(8.0, (win_w - panel_w - 8.0).max(8.0));
            let key = art.map(|art| ImageKey {
                size: ArtSize::Normal,
                ..art
            });
            // The face first: it only borrows the cache, and the image below
            // needs it mutably.
            let built = hovered.and_then(|id| preview_face(&faces, view, &textures, id, key));
            let image = match key {
                Some(key) => textures.get(key, statics, &assets),
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
                    match key {
                        Some(key) => CardLook::art(
                            key,
                            finish_of(statics, Some(key)),
                            crate::cardmat::glow_of(object, crate::cardmat::Offer::NONE),
                        )
                        .with_corner(corner),
                        None => CardLook::back(FinishTreatment::Plain, 0).with_corner(corner),
                    }
                },
                cards.as_mut(),
            );
            let tooltip = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(HAND_BAR_H + 10.0),
                        left: px(left),
                        padding: UiRect::all(px(6)),
                        border_radius: preview_radius(img_w),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(palette::PANEL_LIT),
                    overlay_shadow(),
                    ZIndex(10),
                    Pickable::IGNORE,
                    children![(
                        // Resize handle, bottom right.
                        PreviewResize,
                        Node {
                            position_type: PositionType::Absolute,
                            right: px(4),
                            bottom: px(4),
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
            // A card printed on both sides can be turned over with shift.
            // The frame holds both faces and the turn; a single-faced card
            // gets the frame too, and simply has nothing on its far side, so
            // the shape of the tree does not depend on the card.
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
            commands
                .entity(visual)
                .insert((crate::flip::Side::Front, Visibility::Inherited));
            commands.entity(frame).add_child(visual);
            if let Some(back) = key
                .filter(|_| two_faced(view, hovered))
                .map(|key| ImageKey {
                    face: baylee_client_core::images::Face::Back,
                    ..key
                })
            {
                let art = textures.get(back, statics, &assets);
                let far = spawn_card_art(
                    &mut commands,
                    lang,
                    art,
                    None,
                    img_w,
                    img_h,
                    crate::face::Detail::Full,
                    &fonts,
                    CardLook::art(back, finish_of(statics, Some(back)), 0),
                    cards.as_mut(),
                );
                commands.entity(far).insert((
                    crate::flip::Side::Back,
                    // Hidden until the turn passes the quarter, where the
                    // card is edge-on and the swap cannot be seen.
                    Visibility::Hidden,
                ));
                commands.entity(frame).add_child(far);
            }
            commands.entity(tooltip).add_child(frame);
            commands.entity(root).add_child(tooltip);

            // The speech-bubble tail, pointing at the hovered card.
            let tail = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(HAND_BAR_H + 2.0),
                        left: px(anchor - 9.0),
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

    // ---- the own-board overlay (sliding layer over the ellipse) --------
    if let Some(statics) = duel.statics.as_ref() {
        let overlay = spawn_own_board_overlay(
            &mut commands,
            lang,
            board,
            view,
            statics,
            hovered,
            &selected,
            duel.armed.as_ref(),
            duel.overlay_open,
            duel.overlay_t,
            window_h,
            &mut textures,
            &assets,
            &fonts,
            &faces,
            cards.as_mut(),
        );
        commands.entity(root).add_child(overlay);
    }

    // ---- the stack (left of the rail, when non-empty) --------------------
    if let (false, Some(statics)) = (board.stack.is_empty(), duel.statics.as_ref()) {
        let stack = spawn_stack_panel(
            &mut commands,
            lang,
            board,
            view,
            statics,
            &mut textures,
            &assets,
            &fonts,
            &faces,
            cards.as_mut(),
        );
        commands.entity(root).add_child(stack);
    }

    // ---- the pile chips, and the browser they open ----------------------
    let strip = tray::spawn_pile_strip(&mut commands, lang, &duel.browser, view, &fonts);
    commands.entity(root).add_child(strip);
    if let (true, Some(statics)) = (duel.browser.is_open(), duel.statics.as_ref()) {
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
        );
        commands.entity(root).add_child(tray);
    }
}

/// What an armed deed calls itself, or `None` when the engine no longer
/// offers it.
///
/// Resolved against the *current* `LegalActions` here as well as at the two
/// places that fire it, which is what stops a row drawn a frame ago from
/// offering something that has since been withdrawn. The row simply
/// disappears; the state itself is cleared by the next key or tap, both of
/// which run the same resolution.
fn armed_label(duel: &Duel, lang: Lang, armed: &crate::Armed) -> Option<String> {
    match &armed.deed {
        crate::Deed::Play => duel
            .interaction
            .as_ref()
            .and_then(|i| i.play_card(armed.object))
            .map(|_| Phrase::ArmedPlay.text(lang).to_string()),
        crate::Deed::Ability(action) => super::ability_options(duel, lang, armed.object)?
            .into_iter()
            .find(|o| o.action == *action)
            .map(|o| o.label),
        crate::Deed::Run(plan) => duel
            .reachable
            .contains(&armed.object)
            .then(|| Phrase::ArmedTapAndCast.fill(lang, &[&plan.taps().to_string()])),
    }
}

/// The armed deed as a pair of buttons: the deed itself, and the way back.
///
/// Two buttons and no label between them, because the first one *is* the
/// label — a row that read "Play this card" beside a button called "Send"
/// would be saying the same thing twice and leaving a player to work out
/// which half was the button.
fn spawn_armed(commands: &mut Commands, fonts: &UiFonts, lang: Lang, row: Entity, label: &str) {
    for (action, text, lit, ink) in [
        (
            MenuAction::SendArmed,
            label,
            palette::ACTIVE,
            palette::PANEL,
        ),
        (
            MenuAction::CancelArmed,
            Phrase::ArmedCancel.text(lang),
            palette::PANEL_LIT,
            palette::INK,
        ),
    ] {
        let button = commands
            .spawn((
                MenuButton { action },
                Node {
                    padding: UiRect::axes(px(12), px(5)),
                    border: UiRect::all(px(1)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(lit),
                BorderColor::all(lit),
                soft_shadow(),
                children![(Text::new(text.to_string()), tf(fonts, 13.0), TextColor(ink))],
            ))
            .id();
        commands.entity(row).add_child(button);
    }
}

/// The chip that says this seat is not being asked, and the button out of it.
///
/// It exists because a hold is the one game state with no other symptom: the
/// prompt bar is empty precisely *because* the seat is not being asked, which
/// is exactly what an idle bar looks like. A player who set a hold two turns
/// ago and forgot would watch the game play itself and have nothing on screen
/// to blame. So the state is drawn, and the way out of it sits beside the
/// drawing rather than only on a function key nobody can see.
fn spawn_hold(commands: &mut Commands, fonts: &UiFonts, lang: Lang, row: Entity) {
    let chip = commands
        .spawn((
            Node {
                padding: UiRect::axes(px(12), px(6)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::ACCENT),
            Pickable::IGNORE,
            children![(
                Text::new(Phrase::HoldingPriority.text(lang)),
                tf(fonts, 13.0),
                TextColor(palette::PANEL),
            )],
        ))
        .id();
    let release = commands
        .spawn((
            MenuButton {
                action: MenuAction::ReleaseHold,
            },
            Node {
                padding: UiRect::axes(px(12), px(6)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            soft_shadow(),
            children![(
                Text::new(Phrase::HoldRelease.text(lang)),
                tf(fonts, 13.0),
                TextColor(palette::INK),
            )],
        ))
        .id();
    commands.entity(row).add_child(chip);
    commands.entity(row).add_child(release);
}

/// One arm of the number stepper.
fn spawn_step(commands: &mut Commands, fonts: &UiFonts, delta: i32, glyph: &str) -> Entity {
    commands
        .spawn((
            PromptButton {
                action: PromptAction::Step(delta),
            },
            Node {
                width: px(30),
                height: px(30),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::ACCENT),
            soft_shadow(),
            children![(
                Text::new(glyph.to_string()),
                tf(fonts, 17.0),
                TextColor(palette::PANEL),
            )],
        ))
        .id()
}

/// One player tab: name, life, zone counts; active highlighted, lost
/// grayed out, team color at the border.
#[allow(clippy::too_many_lines)] // the icon+number spans are naturally flat
pub(super) fn spawn_player_tab(
    commands: &mut Commands,
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &baylee_view::SeatView,
    focus: Option<PlayerId>,
    fonts: &UiFonts,
) -> Entity {
    let player = seat.player;
    let name = statics.map_or_else(
        || Phrase::SeatNumbered.fill(lang, &[&player.to_string()]),
        |s| s.seat_name(player).to_string(),
    );
    let team = statics.and_then(|s| s.seats.iter().find(|i| i.player == player)?.team);
    let exile_count = view.exile.get(player.get() as usize).map_or(0, Vec::len);
    let is_active = view.active == player;
    let is_focused = focus == Some(player);
    let has_priority = view.priority == Some(player);

    let is_local = seat.player == view.seat;
    let (background, ink) = if seat.has_lost {
        (palette::PANEL, palette::DEAD)
    } else if is_active {
        (palette::PANEL_LIT, palette::INK)
    } else {
        (palette::PANEL, palette::INK)
    };
    let border_px = if is_active || is_focused { 2.0 } else { 1.0 };

    let marker = if has_priority { "▶ " } else { "" };
    let display = if is_local {
        Phrase::YouNamed.fill(lang, &[&name])
    } else {
        name.clone()
    };
    let counts_color = if seat.has_lost {
        palette::DEAD
    } else {
        palette::MUTED
    };
    let tab = commands
        .spawn((
            PlayerTab { player },
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                padding: UiRect::axes(px(10), px(5)),
                border: UiRect::all(px(border_px)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(background),
            BorderColor::all(if is_active {
                palette::ACTIVE
            } else if is_local {
                palette::ACCENT
            } else {
                team_color(team)
            }),
            soft_shadow(),
            children![(
                // Name and life: name in text font, life with a heart icon.
                Text::new(format!("{marker}{display} ")),
                tf(fonts, 14.0),
                TextColor(if seat.has_lost { palette::DEAD } else { ink }),
                children![
                    (
                        TextSpan::new(glyph::HEART.to_string()),
                        icon_tf(fonts, 11.0),
                        TextColor(if seat.life <= 5 {
                            palette::DANGER
                        } else {
                            palette::ACCENT
                        }),
                    ),
                    (
                        TextSpan::new(format!(" {}", seat.life)),
                        tf(fonts, 14.0),
                        TextColor(if seat.has_lost {
                            palette::DEAD
                        } else if seat.life <= 5 {
                            palette::DANGER
                        } else {
                            ink
                        }),
                    ),
                ],
            ),],
        ))
        .id();

    // Zone counts as icon + number pairs, with experience counters
    // (poison, energy) appearing only when a player actually has them.
    let counts = commands
        .spawn((Text::new(""), tf(fonts, 11.0), TextColor(counts_color)))
        .id();
    commands.entity(tab).add_child(counts);
    let mut span = |icon: char, value: String| {
        let icon_span = commands
            .spawn((
                TextSpan::new(icon.to_string()),
                icon_tf(fonts, 10.0),
                TextColor(counts_color),
            ))
            .id();
        let value_span = commands
            .spawn((
                TextSpan::new(value),
                tf(fonts, 11.0),
                TextColor(counts_color),
            ))
            .id();
        commands.entity(counts).add_child(icon_span);
        commands.entity(counts).add_child(value_span);
    };
    span(glyph::HAND, format!(" {}  ", seat.hand_count));
    span(glyph::LIBRARY, format!(" {}  ", seat.library_count));
    span(glyph::SKULL, format!(" {}  ", seat.graveyard_count));
    span(glyph::EXILE, format!(" {exile_count}"));
    if seat.poison > 0 {
        span(glyph::POISON, format!(" {}", seat.poison));
    }
    if seat.energy > 0 {
        span(glyph::ENERGY, format!(" {}", seat.energy));
    }
    tab
}

/// The own-board overlay: the local player's battlefield as big rounded
/// cards in three lanes, floating above the shared ellipse canvas, with a
/// shadow upwards. Slides down/up (X key or the knob on its top edge).
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)] // panel + knob + lanes are one flat build
pub(super) fn spawn_own_board_overlay(
    commands: &mut Commands,
    lang: Lang,
    board: &baylee_client_core::BoardModel,
    view: &PlayerView,
    statics: &GameStatic,
    hovered: Option<ObjectId>,
    selected: &[ObjectId],
    armed: Option<&crate::Armed>,
    open: bool,
    overlay_t: f32,
    window_h: f32,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    // Spawn already at the current slide position — spawning open and
    // correcting next frame is the battlefield's flicker.
    let open_top = TAB_H;
    let closed_top = closed_overlay_top(window_h);
    let initial_top = closed_top + (open_top - closed_top) * overlay_t;
    let panel = commands
        .spawn((
            OwnBoardOverlay,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(RAIL_W), // 100% minus the phase rail
                top: px(initial_top),
                bottom: px(HAND_BAR_H), // 100% minus tabs and the hand bar
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                // No knob row: the knob floats on the panel's edge, only
                // the button itself is visible. The top gutter is the knob's
                // own height, which is also what makes a *closed* panel show
                // the knob and nothing else — at `KNOB_H` tall its content
                // box is then zero, so the lanes below have nothing to clip.
                padding: UiRect {
                    top: px(KNOB_H),
                    bottom: px(8),
                    left: px(12),
                    right: px(12),
                },
                ..default()
            },
            BackgroundColor(palette::PANEL),
            ZIndex(1),
            overlay_shadow(),
            Pickable::IGNORE,
        ))
        .id();

    // The knob: shallow, centered on the top edge, integrated into the
    // border; the arrow shows the direction the panel will move.
    let knob = commands
        .spawn((
            OverlayKnob,
            Node {
                position_type: PositionType::Absolute,
                top: px(-7),
                left: percent(50),
                margin: UiRect::left(px(-36)),
                width: px(72),
                height: px(KNOB_H),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius {
                    top_left: px(7),
                    top_right: px(7),
                    ..default()
                },
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            children![(
                Text::new((if open { '\u{f078}' } else { '\u{f077}' }).to_string()),
                icon_tf(fonts, 9.0),
                TextColor(palette::MUTED),
            )],
        ))
        .id();
    commands.entity(panel).add_child(knob);

    let Some(pod) = board.pods.iter().find(|p| p.is_local) else {
        return panel;
    };

    // The lanes go in their own box, and that box clips vertically.
    //
    // A closed panel is exactly `KNOB_H` tall, and a card in it is
    // `OVERLAY_CARD_H` — so without this the tops of your own permanents
    // stand above the hand bar whenever the overlay is shut, which reads as
    // cards left behind by the ones you played. Clipped on `y` only: a row
    // that outgrows the panel sideways is a different question, and hiding
    // its tail would be the lie rule 3 is about.
    let lanes = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                // Without this the clip below is decoration. A flex item's
                // automatic minimum size is its *content*, so the box grew to
                // hold a full card row and then clipped nothing — which is
                // why the first attempt at this fix changed the picture by a
                // few pixels and nothing else.
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                overflow: Overflow::clip_y(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(lanes);

    for lane in &pod.lanes {
        if lane.groups.is_empty() {
            continue;
        }
        let row = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(6),
                    height: px(OVERLAY_CARD_H),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        for group in &lane.groups {
            let is_selected = group.members.iter().any(|m| selected.contains(m));
            let is_hovered = hovered == Some(group.representative);
            let shadow = if is_selected || is_hovered {
                BoxShadow::new(
                    palette::ACCENT,
                    Val::Px(0.0),
                    Val::Px(0.0),
                    Val::Px(0.0),
                    Val::Px(8.0),
                )
            } else {
                soft_shadow()
            };
            let object = view.object(group.representative);
            let built = object.and_then(|o| faces.object(o, textures, group.art));
            // A token has no printing at all, so its face is the only thing
            // there is to draw — before this the overlay skipped it entirely.
            let image = match group.art {
                Some(art) => textures.get(art, statics, assets),
                None => textures.card_back(),
            };
            if built.is_none() && group.art.is_none() {
                continue;
            }
            let visual = spawn_card_art(
                commands,
                lang,
                image,
                built.as_ref(),
                OVERLAY_CARD_W,
                OVERLAY_CARD_H,
                crate::face::Detail::Compact,
                fonts,
                {
                    // The same claims the table draws, drawn the same way:
                    // what the rules made the card, and what the player could
                    // do with it or has just said they will. The overlay is
                    // where a seat looks at its own board, so it is the last
                    // place those cues should be missing.
                    let glow = crate::cardmat::glow_of(
                        view.object(group.representative),
                        crate::cardmat::Offer::on(armed, &group.members, group.activatable),
                    );
                    // And its body, for the same reason: the overlay draws the
                    // same permanent through the same shader, so a 2/2 that is
                    // a 4/4 on the table must not be a 2/2 here.
                    let corner = baylee_client_core::cardplate::Corner::of(group);
                    match group.art {
                        Some(art) => CardLook::art(art, finish_of(statics, Some(art)), glow)
                            .with_corner(corner),
                        None => CardLook::back(FinishTreatment::Plain, glow).with_corner(corner),
                    }
                },
                cards.as_deref_mut(),
            );
            let card = commands
                .spawn((
                    HandCardVisual {
                        object: group.representative,
                    },
                    Node {
                        width: px(OVERLAY_CARD_W),
                        height: px(OVERLAY_CARD_H),
                        border_radius: card_radius(OVERLAY_CARD_W),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    shadow,
                    children![(
                        // Count chip for grouped stacks.
                        Text::new(if group.count() > 1 {
                            format!("×{}", group.count())
                        } else {
                            String::new()
                        }),
                        tf(fonts, 12.0),
                        TextColor(palette::INK),
                        Node {
                            position_type: PositionType::Absolute,
                            right: px(3),
                            bottom: px(2),
                            ..default()
                        },
                    ),],
                ))
                .id();
            commands.entity(card).add_child(visual);
            commands.entity(row).add_child(card);
        }
        commands.entity(lanes).add_child(row);
    }
    panel
}

/// Slides the own-board overlay between its raised and its down position.
/// Raised: pinned under the tab bar. Down: slid beneath the hand (which
/// stays on top), with only the knob peeking above the hand bar so there
/// is always a way back.
pub fn animate_overlay(
    time: Res<Time>,
    mut duel: ResMut<Duel>,
    windows: Query<&Window>,
    mut panels: Query<&mut Node, With<OwnBoardOverlay>>,
) {
    let target = if duel.overlay_open { 1.0 } else { 0.0 };
    let Ok(window) = windows.single() else {
        return;
    };
    // The `top` is recomputed every frame, so window resizes stay honest
    // even when the animation has settled.
    if (duel.overlay_t - target).abs() >= f32::EPSILON {
        let step = time.delta_secs() * 5.0;
        duel.overlay_t = if (target - duel.overlay_t).abs() <= step {
            target
        } else {
            duel.overlay_t + (target - duel.overlay_t).signum() * step
        };
    }
    let open_top = TAB_H;
    let closed_top = closed_overlay_top(window.height());
    let top = closed_top + (open_top - closed_top) * duel.overlay_t;
    for mut node in &mut panels {
        node.top = px(top);
    }
}

/// Whether the hovered object is a card printed on both sides.
///
/// The view says which face is up, not how many there are, so the answer
/// comes from the registry the client already links for ability labels and
/// mana sources. A token or a face-down permanent has no card and therefore
/// no back.
/// The line that says how a choice is answered, when it is answered by
/// clicking something rather than by pressing a button.
///
/// `None` for every choice that draws its own answers, so a hint never
/// appears next to a row of buttons that already says what to do. A creature
/// type is the exception, because there the hint is not about where to click
/// — it is about the box, and a list cut to twelve of three hundred and fifty
/// says nothing about typing on its own.
const fn pick_hint(prompt: &Prompt) -> Option<Phrase> {
    match prompt {
        // The seat's own hand, which the engine does not enumerate because it
        // is already private -- `Interaction::selectable` is empty for both.
        Prompt::Discard { .. } | Prompt::BottomCards { .. } => Some(Phrase::HintClickHand),
        Prompt::ChooseCards { .. } | Prompt::ChooseTargets { .. } | Prompt::LegendRule => {
            Some(Phrase::HintClickBoard)
        }
        Prompt::ChooseSubtype { .. } => Some(Phrase::HintTypeToFilter),
        _ => None,
    }
}

fn two_faced(view: &PlayerView, hovered: Option<ObjectId>) -> bool {
    hovered
        .and_then(|id| view.object(id))
        .and_then(|object| object.card.as_ref())
        .and_then(|card| baylee_cards::by_index(card.index))
        .is_some_and(|def| def.faces.len() > 1)
}
