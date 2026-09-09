//! The retained HUD tree: the seat tabs, the prompt slip, the stack, and
//! the one system that rebuilds all of it.
//!
//! Rebuilt only when [`HudRevision`] says something it draws has changed —
//! a rebuild per frame would cost more than the whole table does.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::commanderdamage;
use baylee_client_core::interaction::Prompt;

/// How wide the commander-damage track is drawn, in logical pixels.
///
/// The same in every seat's tab, whatever that seat's worst source is,
/// because the bar's whole job is to be comparable: it stands for twenty-one
/// (`commanderdamage::LETHAL`) and nothing else, so a half-full bar means the
/// same thing under every name at the table.
const TRACK_W: f32 = 52.0;

/// The gap between two answers on the prompt slip, in logical pixels.
///
/// Small, and it has to be: the answers divide the sheet between them, so
/// every pixel here is a pixel off each button. Wide enough that two brass
/// edges never touch, narrow enough that the row still reads as one control
/// with parts rather than as scattered buttons.
const BUTTON_GAP: f32 = 8.0;

/// The narrowest the prompt slip is drawn.
///
/// The sheet takes its width from its longest line, and its shortest question
/// ("You have priority") is shorter than the two answers under it. Without a
/// floor the slip shrinks to the words and the buttons are squeezed into a
/// sliver; with one, every question is asked on a sheet of a recognisable
/// size, which is also what stops the slip jumping about between steps.
const SLIP_MIN_W: f32 = 380.0;

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
    let prompt = ending.or_else(|| duel.interaction.as_ref().map(|i| i.prompt().headline(lang)));
    // A refusal used to *stand in* for the headline, which meant it was only
    // ever seen when nothing was being asked — and the engine refuses an
    // answer precisely while a question is standing. The player clicked, the
    // bar went on saying "Choose a target", and nothing else happened. It is
    // its own line now, under whatever the bar was already saying.
    let error = duel.last_error.clone();
    // The connection, when it has something to say. Drawn in the same bar
    // rather than in a banner of its own because that is where this client
    // already speaks to the player, and above the rest of it because a table
    // that cannot hear you makes every other line on the bar moot.
    let link_note = duel.link_note;
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
    let ability_menu = duel.ability_menu;
    let ability_pick = duel.ability_pick;
    let focus = duel.focus;
    let preview_scale = settings.preview_scale;
    let browser = (
        duel.browser.is_open(),
        duel.browser.tab(),
        duel.browser.filter().to_string(),
        duel.browser.is_typing(),
    );
    let menu = (duel.can_offer_draw(), duel.concede_armed);
    let armed_deed = duel.armed.clone();
    let number = duel
        .interaction
        .as_ref()
        .and_then(|i| matches!(i.prompt(), Prompt::ChooseNumber { .. }).then(|| i.number()));

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

    // ---- under the seats: the phase rail, twelve steps left to right ---
    let rail = spawn_phase_rail(
        &mut commands,
        lang,
        view,
        &orders,
        &fonts,
        duel.statics.as_ref(),
    );
    commands.entity(root).add_child(rail);

    // ---- the prompt slip: the question, and the answers to it -------------
    //
    // Centred over the near edge of the player's own board, and a sheet of
    // parchment rather than a panel. Both of those are the same argument. It
    // used to sit in the bottom-right corner in 88%-black at 13 px — the one
    // thing on screen that has to be answered, drawn as the least prominent
    // thing on it, in the corner furthest from where a player's eyes are
    // (their own hand, and the board above it). A question and the hand it is
    // answered from are now the same place to look.
    if prompt.is_some() || error.is_some() || link_note.is_some() {
        let waiting = !duel.is_my_turn_to_act();
        // The centring row spans the whole window and is ignored by the
        // pointer, so it takes nothing away from the board it lies over; only
        // the slip inside it is a surface.
        let slip_row = commands.spawn((slip_row_node(), Pickable::IGNORE)).id();
        let slip = commands.spawn((
            Node {
                max_width: px(620),
                min_width: px(SLIP_MIN_W),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(7),
                padding: UiRect::axes(px(22), px(13)),
                border: UiRect::all(px(1)),
                border_radius: sheet_radius(),
                ..default()
            },
            BackgroundColor(palette::PARCHMENT),
            BorderColor::all(palette::PARCHMENT_EDGE),
            sheet_shadow(),
        ));
        let bar = slip.id();
        commands.entity(slip_row).add_child(bar);
        // First child, and a child rather than the slip's own image: see
        // [`sheet_surface`] for the ring of flat parchment that was.
        if let Some(sheets) = sheets.as_deref() {
            let surface = commands.spawn(sheet_surface(sheets)).id();
            commands.entity(bar).add_child(surface);
        }
        // Above the headline, because a table this client cannot reach makes
        // every other line in the bar moot: the question standing there was
        // asked before the socket went, and answering it will not arrive.
        if let Some(phrase) = link_note {
            let line = slip_line(
                &mut commands,
                &fonts,
                phrase.text(lang),
                15.0,
                palette::INK_DANGER,
            );
            commands.entity(bar).add_child(line);
        }

        if let Some(text) = prompt {
            let ink = if waiting {
                palette::SLIP_SOFT
            } else {
                palette::SLIP_INK
            };
            let headline = slip_line(&mut commands, &fonts, &text, 18.0, ink);
            commands.entity(bar).add_child(headline);
        }

        // What the engine said no to. It survives until this seat submits
        // something else (`Duel::submit` clears it), so it is still there to
        // read after the click that earned it — and it is drawn even while
        // another seat is being asked, because a refusal is an answer to
        // something *this* player did.
        if let Some(text) = error {
            let line = slip_line(&mut commands, &fonts, &text, 13.0, palette::INK_DANGER);
            commands.entity(bar).add_child(line);
        }

        // A choice that is answered by clicking has to say so. The prompt
        // bar used to draw "Discard 1 card(s)" and stop: no button, because
        // nothing is submittable until something is picked, and no hint,
        // because none existed. A player who did not already know to click
        // their hand had no way to find out.
        if let Some(hint) = duel
            .interaction
            .as_ref()
            .filter(|i| !waiting && i.selected().next().is_none())
            .and_then(|i| pick_hint(&i.prompt()))
        {
            let line = slip_line(
                &mut commands,
                &fonts,
                hint.text(lang),
                12.0,
                palette::SLIP_SOFT,
            );
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
            let aim = slip_line(&mut commands, &fonts, &line, 13.0, palette::SLIP_SOFT);
            commands.entity(bar).add_child(aim);
        }

        // ---- combat: what is coming at whom --------------------------------
        //
        // Unlike the aim above, this is not about a declaration this seat is
        // making, so it is not filtered on `waiting`: an attack aimed at you
        // while the other side is still choosing blockers is exactly the
        // thing you need to be able to read.
        if let Some((line, threatened)) =
            incoming_line(view, duel.interaction.as_ref(), duel.statics.as_ref(), lang)
        {
            let ink = if threatened {
                palette::INK_DANGER
            } else {
                palette::SLIP_SOFT
            };
            let incoming = slip_line(&mut commands, &fonts, &line, 13.0, ink);
            commands.entity(bar).add_child(incoming);
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
                    TextColor(palette::PARCHMENT_INK),
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
        // Priority is not confirmed, it is *passed*, and the two words are not
        // interchangeable on a button. "OK" acknowledges something that has
        // already happened; a player reading it under "You have priority" was
        // being told to dismiss a window rather than invited to act, and said
        // so. `PromptAction::Confirm` still carries it — the action was always
        // right, only its label was wrong.
        // Two sizes of the same decision. "Skip turn" is where the rail.s
        // fast-forward button went: it is the answer to this window and to
        // every window until this turn is over, and a player deciding to sit
        // one out should find it under the question rather than on a strip in
        // the corner.
        let pass_answer = [
            (PromptAction::Confirm, Phrase::PassPriority.text(lang)),
            (PromptAction::SkipTurn, Phrase::SkipTheTurn.text(lang)),
        ];
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
                Some(baylee_engine::choice::Pending::Priority { .. }) => &pass_answer,
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
            // The answers share the sheet's width, whatever there are of
            // them: two, three or one, each takes the same fraction of it.
            // The row could shrink to its labels instead — it used to — and
            // then "Pass priority" and "Skip turn" sat in a huddle in the
            // middle of a sheet with two inches of parchment either side of
            // it, and a mulligan's two answers were a different size from
            // combat's three. A button whose width says nothing is a button
            // whose position says nothing.
            let row = commands.spawn((answer_row_node(), Pickable::IGNORE)).id();
            // The first answer is the one the question is asking for — keep,
            // confirm, pass — and it is the only one drawn in brass. The rest
            // are the same button in the sheet's own colour, because two
            // equally loud answers make a player read both before finding out
            // which one the sheet meant.
            for (i, (action, label)) in answers.iter().enumerate() {
                let lead = i == 0;
                let rest = if lead {
                    palette::BRASS
                } else {
                    palette::SLIP_GHOST
                };
                let button = commands
                    .spawn((
                        PromptButton { action: *action },
                        answer_node(),
                        BackgroundColor(rest),
                        BorderColor::all(if lead {
                            palette::BRASS
                        } else {
                            palette::PARCHMENT_EDGE
                        }),
                        soft_shadow(),
                        // The same component every other button in the client
                        // carries: `ambience::feel` leans it towards the
                        // pointer, sinks it under a press and owns its
                        // `BackgroundColor` from the first frame on.
                        Feel::new(rest),
                        children![(
                            Text::new(*label),
                            tf(&fonts, 13.0),
                            TextColor(if lead {
                                palette::PARCHMENT_INK
                            } else {
                                palette::PARCHMENT_SOFT
                            }),
                            // A label is a `Node`, and a node under the
                            // pointer is what the pointer is *over*: without
                            // this the button only ever lit up when the
                            // pointer was in its padding, and went dead the
                            // moment it crossed the word it is named after.
                            // Measured, not guessed — hovering the padding
                            // moved 155 levels and hovering the word moved
                            // none.
                            Pickable::IGNORE,
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
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.06)),
                    BorderColor::all(palette::BRASS),
                    Pickable::IGNORE,
                ))
                .id();
            // A caret with nothing before it, so an empty box still reads as
            // somewhere to type rather than as a blank panel.
            let text = commands
                .spawn((
                    Text::new(format!("{}_", duel.subtype_filter)),
                    tf(&fonts, 14.0),
                    TextColor(palette::PARCHMENT_INK),
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
                            palette::BRASS
                        } else {
                            Color::srgba(0.0, 0.0, 0.0, 0.05)
                        }),
                        BorderColor::all(if on {
                            palette::BRASS
                        } else {
                            palette::PARCHMENT_EDGE
                        }),
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
                            TextColor(palette::PARCHMENT_INK),
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
                            palette::BRASS
                        } else {
                            Color::srgba(0.0, 0.0, 0.0, 0.05)
                        }),
                        BorderColor::all(if picked {
                            palette::BRASS
                        } else {
                            palette::PARCHMENT_EDGE
                        }),
                        soft_shadow(),
                        children![(
                            Text::new(option.label.clone()),
                            tf(&fonts, 13.0),
                            TextColor(palette::PARCHMENT_INK),
                        )],
                    ))
                    .id();
                commands.entity(row).add_child(button);
            }
            commands.entity(bar).add_child(row);
        }
        commands.entity(root).add_child(slip_row);
    }

    // ---- the mana pool, opposite the prompt bar --------------------------
    //
    // The one zone with no card in it, and until now the one zone with no
    // place on screen. That absence hid a defect rather than merely being
    // untidy: a land with two mana abilities taps for whichever one the
    // client's planner can read, and with nothing drawn there was no way to
    // see which had fired — Jasmine Dragon Tea Shop made `{C}` every time and
    // looked exactly like a land making the Ally mana it was tapped for.
    //
    // Left, because the prompt bar is right and the two must never push each
    // other around; at the prompt bar's height, because that band is already
    // where this client says what is going on. Drawn while the seat has
    // something to answer even when empty, so it is a *place* a player learns
    // rather than a badge that appears and vanishes — and hidden entirely
    // when the seat is only watching, since floating mana it cannot spend is
    // one more thing in the way.
    {
        let pool = view
            .seat(view.seat)
            .map(|s| s.mana_pool)
            .unwrap_or_default();
        let floating = baylee_client_core::manapool::row(&pool);
        if !floating.is_empty() || duel.is_my_turn_to_act() {
            let bar = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(HAND_BAR_H + 10.0),
                        left: px(12),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(8),
                        padding: UiRect::axes(px(12), px(7)),
                        border_radius: btn_radius(),
                        ..default()
                    },
                    BackgroundColor(palette::PANEL),
                    soft_shadow(),
                    Pickable::IGNORE,
                ))
                .id();
            let label = commands
                .spawn((
                    Text::new(Phrase::ManaPool.text(lang).to_string()),
                    tf(&fonts, 11.0),
                    TextColor(palette::MUTED),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(bar).add_child(label);
            if floating.is_empty() {
                // An em dash rather than a row of zeroes: "nothing floating"
                // is one fact, not six.
                let none = commands
                    .spawn((
                        Text::new("\u{2014}".to_string()),
                        tf(&fonts, 15.0),
                        TextColor(palette::DEAD),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(bar).add_child(none);
            }
            for entry in floating {
                let group = commands
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(3),
                            // A restriction is drawn as a rim round the pair,
                            // because the symbol itself has to keep meaning
                            // its colour: this mana *is* white, it simply
                            // cannot pay for everything white pays for.
                            padding: UiRect::axes(px(4), px(2)),
                            border: UiRect::all(px(if entry.restricted { 1.0 } else { 0.0 })),
                            border_radius: btn_radius(),
                            ..default()
                        },
                        BorderColor::all(palette::ACTIVE),
                        Pickable::IGNORE,
                    ))
                    .id();
                let pip = crate::manaui::spawn_pip(&mut commands, &fonts, entry.pip, 18.0);
                // The count as a numeral, always — colour alone must not
                // carry meaning, and five discs in a row is a number the
                // player has to stop and count.
                let count = commands
                    .spawn((
                        Text::new(format!("\u{00d7}{}", entry.count)),
                        tf(&fonts, 13.0),
                        TextColor(palette::INK),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(group).add_children(&[pip, count]);
                commands.entity(bar).add_child(group);
            }
            commands.entity(root).add_child(bar);
        }
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
        if let Some((art, anchor)) = preview_anchor(
            board,
            view,
            hovered,
            layout,
            duel.hand_scroll,
            duel.hovered_at,
        ) {
            let scale = settings.preview_scale.clamp(0.5, 1.75);
            let img_w = 308.0 * scale;
            let img_h = img_w * 88.0 / 63.0;
            let window = windows.single().map_or(Vec2::new(1200.0, 800.0), |w| {
                Vec2::new(w.width(), w.height())
            });
            // The panel is the picture plus its six pixels of padding on
            // every side.
            let panel = Vec2::new(img_w, img_h) + Vec2::splat(12.0);
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
                    match shown {
                        Some(shown) => CardLook::art(
                            shown,
                            finish_of(statics, Some(shown)),
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
                        top: px(place.y),
                        left: px(place.x),
                        padding: UiRect::all(px(6)),
                        border_radius: preview_radius(img_w),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(palette::PANEL_LIT),
                    upward_shadow(),
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
            commands.entity(root).add_child(tooltip);

            // The speech-bubble tail, pointing down at the hovered card —
            // and only for a hand card, which is the only one the tail can
            // point *at*. A panel standing beside a permanent needs none: it
            // is already next to the thing it describes, and a caret aimed
            // down into the hand bar from there would name a card at random.
            if let PreviewAt::Hand(x) = anchor {
                let tail = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: px(HAND_BAR_H + 2.0),
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
            sheets.as_deref(),
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
    for (action, text, lit, edge, ink) in [
        (
            MenuAction::SendArmed,
            label,
            palette::BRASS,
            palette::BRASS,
            palette::PARCHMENT_INK,
        ),
        (
            MenuAction::CancelArmed,
            Phrase::ArmedCancel.text(lang),
            Color::NONE,
            palette::PARCHMENT_EDGE,
            palette::PARCHMENT_SOFT,
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
                BorderColor::all(edge),
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
/// The row the prompt slip's answers stand in.
///
/// Full width, because the answers divide the sheet between them. The row
/// used to shrink to its labels, and then "Pass priority" and "Skip turn"
/// huddled in the middle of a sheet with parchment either side of them, and a
/// mulligan's two answers came out a different size from combat's three. A
/// button whose width says nothing is a button whose position says nothing.
pub(super) fn answer_row_node() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Row,
        column_gap: px(BUTTON_GAP),
        margin: UiRect::top(px(2)),
        ..default()
    }
}

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

/// One line of the prompt slip's prose.
///
/// Four things at once, and they are one decision rather than four. The slip
/// is the sheet a question is *written* on, so its lines are set in the
/// italic of the same family, cast the faint warm shadow a letter lying on
/// parchment casts, carry a little of the sheet through the ink, and hand
/// their bracketed asides to a grey — a key to press or a count the board
/// already shows is not part of the sentence, and reading it as if it were
/// makes every question longer than it is.
///
/// The split is [`baylee_client_core::prose::bracketed`], which is where the
/// test for it lives; here it becomes one [`TextSpan`] per run. The root
/// carries the shadow, because a shadow is per text block rather than per
/// span, and an empty root string draws nothing of its own.
fn slip_line(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    ink: Color,
) -> Entity {
    let line = commands
        .spawn((
            Text::default(),
            tf_italic(fonts, size),
            TextColor(ink),
            TextShadow {
                offset: Vec2::new(0.0, 1.0),
                color: palette::SLIP_SHADOW,
            },
        ))
        .id();
    for (run, aside) in baylee_client_core::prose::bracketed(text) {
        let span = commands
            .spawn((
                TextSpan::new(run.to_string()),
                tf_italic(fonts, size),
                TextColor(if aside { palette::SLIP_ASIDE } else { ink }),
            ))
            .id();
        commands.entity(line).add_child(span);
    }
    line
}

pub(super) fn slip_row_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        bottom: px(HAND_BAR_H + 12.0),
        left: px(0),
        right: px(0),
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

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
            BackgroundColor(palette::BRASS),
            soft_shadow(),
            Feel::new(palette::BRASS),
            children![(
                Text::new(glyph.to_string()),
                tf(fonts, 17.0),
                TextColor(palette::PARCHMENT_INK),
                // See [`answer_node`]: a label is a node, and a node under the
                // pointer is what the pointer is over.
                Pickable::IGNORE,
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

    // The second life total (CR 903.10a), and shown like one: a bar rather
    // than a row of numbers, because the question is "how close am I to dying
    // to this" and not "what do these add up to" — they add up to nothing the
    // rules recognise.
    //
    // Only when a commander has actually connected, the rule poison and energy
    // follow above. A bar sitting at zero under all eight seats every game
    // would be noise on the one screen that has no room for any.
    if let Some(track) = commanderdamage::Track::of(&seat.commander_damage) {
        let color = if track.danger {
            palette::DANGER
        } else {
            counts_color
        };
        let row = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(4.0),
                    ..default()
                },
                children![(
                    Text::new(glyph::COMMAND.to_string()),
                    icon_tf(fonts, 10.0),
                    TextColor(color),
                )],
            ))
            .id();
        commands.entity(tab).add_child(row);

        // Fixed width, and the same width in every seat's tab: the eye learns
        // what full looks like once, and then reads every other seat against
        // it. A bar scaled to its own worst source would make two seats with
        // very different problems look identical.
        let bar = commands
            .spawn((
                Node {
                    width: px(TRACK_W),
                    height: px(4.0),
                    border_radius: BorderRadius::all(px(2.0)),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
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
            ))
            .id();
        commands.entity(bar).add_child(fill);

        // One tick per other commander, on the same scale. They say how many
        // more clocks are running and how far along each is; which commander
        // is which is the tooltip's answer, the same half this panel already
        // gives for a counter's colour.
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
                ))
                .id();
            commands.entity(bar).add_child(tick);
        }

        let worst = commands
            .spawn((
                Text::new(track.worst.to_string()),
                tf(fonts, 11.0),
                TextColor(color),
            ))
            .id();
        commands.entity(row).add_child(worst);
    }
    tab
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

fn two_faced(view: &PlayerView, hovered: Option<ObjectId>) -> bool {
    hovered
        .and_then(|id| view.object(id))
        .and_then(|object| object.card.as_ref())
        .and_then(|card| baylee_cards::by_index(card.index))
        .is_some_and(|def| def.faces.len() > 1)
}
