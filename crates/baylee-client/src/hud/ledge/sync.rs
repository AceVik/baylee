//! The rebuild: `sync_ledge`, and the hand tools.

#[allow(clippy::wildcard_imports)] // the ledge's shared vocabulary
use super::*;

/// Builds what stands on the shelf, when what is written on it changes.
///
/// After `sync_overlay`, because the shelf it fills is spawned there — and
/// the emptiness check below is what makes that ordering a preference rather
/// than a requirement: a shelf that has just been built afresh is filled even
/// when nothing in the revision moved.
#[allow(clippy::too_many_lines)] // one retained-UI rebuild, sectioned by comments
#[allow(clippy::too_many_arguments)]
pub fn sync_ledge(
    mut commands: Commands,
    duel: Res<Duel>,
    mut revision: ResMut<LedgeRevision>,
    mut layout: ResMut<LedgeLayout>,
    shelf: Query<(Entity, Option<&Children>), With<LedgeShelf>>,
    // Two exemptions from the rebuild below, and one query for both: the
    // pool's column, which outlives a rebuild so a mana can be drawn arriving
    // (`ledge/pool.rs`), and the two casts, which are spawned with the shelf
    // and never change.
    retained: Query<(), Retained>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    prefs: Res<crate::prefs::Prefs>,
    texts: Res<crate::cardtext::CardTexts>,
    windows: Query<&Window>,
) {
    let Ok((shelf, standing)) = shelf.single() else {
        return;
    };
    let lang = Lang::of(&settings.lang);
    // A finished game is the one question this shelf does not answer: who won
    // is the end screen's whole subject, set at four times the size. The
    // column simply empties — see `sync_overlay`'s own note, which this is
    // the other half of.
    let over = duel.ending().is_some();
    let waiting = !duel.is_my_turn_to_act();
    let elsewhere = duel.browser.answers_here(duel.interaction.as_ref());
    // Under a decision sheet the question is the sheet's head, and the shelf
    // under it is the sheet's foot: its answers, without the sentence again.
    let prompt = if super::drawer::sheet_up(&duel) {
        None
    } else {
        shelf_headline(&duel, lang, &texts)
    };
    #[allow(clippy::cast_possible_truncation)]
    let window_w = windows.single().map_or(1200, |w| w.width() as i32);
    let next = LedgeRevision {
        chosen_index: duel
            .interaction
            .as_ref()
            .and_then(baylee_client_core::Interaction::chosen_index),
        decision_id: duel
            .interaction
            .as_ref()
            .and_then(baylee_client_core::Interaction::decision_id),
        hand_order: duel.hand_order,
        seq: duel.board.as_ref().map(|b| b.seq),
        over,
        prompt,
        error: duel.last_error.clone().filter(|_| !over),
        link_note: duel.link_note.filter(|_| !over),
        clock: duel.clock.shown().is_some() && !over,
        waiting,
        elsewhere,
        selected: duel
            .interaction
            .as_ref()
            .map(|i| i.selected().collect())
            .unwrap_or_default(),
        selected_players: duel
            .interaction
            .as_ref()
            .map(|i| i.selected_players().collect())
            .unwrap_or_default(),
        declared: duel
            .interaction
            .as_ref()
            .map_or(0, baylee_client_core::Interaction::declared),
        armed: duel.armed.clone(),
        cast_menu: duel.cast_menu.is_some() && !over,
        holdable: duel.can_hold_for_stack(),
        can_offer_draw: duel.can_offer_draw(),
        concede_armed: duel.concede_armed,
        menu_open: duel.game_menu,
        priority_held: duel.priority_held(),
        autopilot: duel.autopilot.is_some(),
        lang: Some(lang),
        // Compared in place below and cloned only for a rebuild: a keymap is
        // a map of lists, and copying it to compare it was ~70 allocations
        // on every frame of a table at rest.
        keys: None,
        window_w,
    };
    // The second half of the gate is what covers a shelf that was spawned
    // afresh with the revision still describing the tree before it. Everything
    // spawned *with* the shelf is exempt from the rebuild below and is
    // therefore also not evidence that the rebuild has run, so the question is
    // whether anything else is standing there.
    let filled = standing.is_some_and(|c| c.iter().any(|child| retained.get(child).is_err()));
    // The stored keymap is lifted out for the comparison and put back, past
    // change detection: comparing is not a rebuild.
    let kept = revision.bypass_change_detection().keys.take();
    let same = *revision == next && kept.as_ref() == Some(prefs.keymap());
    revision.bypass_change_detection().keys = kept;
    if same && filled {
        return;
    }
    *revision = LedgeRevision {
        keys: Some(prefs.keymap().clone()),
        ..next
    };

    for child in standing.into_iter().flatten() {
        // Everything the shelf was not spawned with. The two casts are
        // exempt because they are the shelf's own elevation and answer to
        // nothing this system knows about. The mana pool used to be the third
        // and is no longer a child of this node at all — see `ledge/pool.rs`,
        // which says why it had to outlive this rebuild long before it became
        // a strip: mana arrives and is spent *inside* one question, and §4.1
        // wants that drawn arriving.
        if retained.get(*child).is_err() {
            commands.entity(*child).despawn();
        }
    }

    let tools = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(EDGE),
                top: px(LEDGE_PAD_Y - LIP + 2.0),
                column_gap: px(TOOL_GAP),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(shelf).add_child(tools);
    let orders = if window_w >= WIDE_ENOUGH_FOR_FIVE {
        crate::hand_order::HandOrder::ALL.to_vec()
    } else {
        vec![duel.hand_order]
    };
    for order in orders {
        let label = if window_w >= WIDE_ENOUGH_FOR_FIVE {
            order.label(lang).to_string()
        } else {
            format!("Hand: {} ›", order.label(lang))
        };
        let weight = if order == duel.hand_order {
            Weight::Candle
        } else {
            Weight::Secondary
        };
        let button = hand_tool(&mut commands, &fonts, &label, order, weight);
        commands.entity(button).insert(MenuButton {
            action: MenuAction::SortHand(if window_w >= WIDE_ENOUGH_FOR_FIVE {
                order
            } else {
                order.next()
            }),
        });
        commands.entity(tools).add_child(button);
    }

    // The middle is built first because it is the only one that knows how
    // wide it is, and how wide it is decides the arrangement of all three.
    let answers = answers_for(&duel, lang, over, waiting, elsewhere);
    let armed = duel
        .armed
        .as_ref()
        .filter(|_| !over)
        .and_then(|a| super::overlay::armed_label(&duel, lang, a));
    let sentence = revision
        .link_note
        .map(|note| (note.text(lang).to_string(), true))
        .or_else(|| {
            revision
                .error
                .as_ref()
                .map(|refusal| (refusal.text(lang), true))
        })
        // An armed card says what it is about to do on the button itself, so
        // the question above it would be the same sentence a second time —
        // §6: the shelf never shows two sentences, and the armed row is the
        // one state that takes the sentence away rather than replacing it.
        //
        // A running hold replaces it instead, and that is the whole of §4.4:
        // "why is nobody asking me?" is a question about the middle, so it is
        // answered in the middle, where the question would have been.
        .or_else(|| {
            if holding(&duel, over, waiting) {
                Some(Phrase::HoldingPriority.text(lang).to_string())
            } else {
                revision.prompt.clone()
            }
            .filter(|_| armed.is_none())
            .map(|text| (text, false))
        });

    let caps = keys_for(&prefs, &answers, armed.is_some(), duel.priority_held());
    let caps_w: f32 = caps.iter().flatten().map(|c| cap_width(c) + CAP_GAP).sum();
    let (clock, clocked) = clock_placement(&duel, revision.clock, armed.is_some(), &answers);
    let mid = mid_width(
        sentence.as_ref().map(|(t, _)| t.as_str()),
        clock,
        &answers,
        &caps,
    );
    #[allow(clippy::cast_precision_loss)]
    let arrangement = baylee_client_core::ledge::arrange(
        window_w as f32,
        baylee_client_core::ledge::Columns {
            left: tools_reserved(window_w),
            mid,
            right: RIGHT_RESERVED,
        },
        caps_w,
    );
    // The drawer stands over the question, so it has to be told where the
    // question ended up. Written here rather than read from the node, because
    // a `Node`'s padding is bevy_ui's to lay out and would be a frame stale by
    // the time anything read it back.
    layout.mid_x = arrangement.mid_x;
    layout.window_w = window_w;

    // Two, not three. The left column is gone: the mana pool hangs off the
    // shelf's left end as a strip of its own now, a child of the overlay's
    // root rather than of this node. See `ledge/pool.rs`.
    let columns = [
        column_node(Side::Mid(arrangement.mid_x, window_w)),
        column_node(Side::Right),
    ]
    .map(|node| commands.spawn(node).id());
    commands.entity(shelf).add_children(&columns);

    ways_out(&mut commands, &fonts, columns[1], &revision);

    let middle = columns[0];

    // `Split` is the rung that sends the sentence into the drawer, and there
    // is no drawer yet. Until there is, it draws what `Compact` draws: a
    // question the player has already read is worth less than the buttons,
    // which is exactly why `Split` gives it up — but dropping it on the floor
    // instead of putting it somewhere is not the same trade.
    // Left of the sentence, because a clock is read before the words it is
    // about. It is built here and written by `count_down_the_decision`, which
    // is why it starts empty: one frame with no digits is invisible, and a
    // revision that carried the digits would rebuild the shelf once a second.
    //
    // Only where no button on the row is what the clock presses: otherwise
    // the seconds are in that button (#258), and one number is drawn once.
    if clock == Clock::Beside {
        let cell = commands
            .spawn((
                DecisionClockLabel,
                Text::default(),
                // A readout and not prose, so the shelf's own bold rather
                // than the slant the question is written in. Flat: one ink at
                // sixty seconds and the same ink at one. What marks the two
                // moments is `Cue::ClockLow`, which is a sound and does not
                // have to compete with a board for the eye.
                super::tf_bold(&fonts, SENTENCE_PT),
                TextColor(palette::LEDGE_SOFT),
                Node {
                    width: px(clock_width()),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(middle).add_child(cell);
    }

    let shows_sentence = arrangement.density.shows_sentence()
        || arrangement.density == baylee_client_core::ledge::Density::Split;
    if let Some((text, alarming)) = sentence.filter(|_| shows_sentence) {
        // `LEDGE_SOFT` and not `DIALOG_SOFT`: this sentence stands on the
        // shelf's own ground, which is no longer opaque. The constant says
        // what that costs and why it is only for ink standing here.
        let ink = if alarming {
            palette::DANGER
        } else if waiting {
            palette::LEDGE_SOFT
        } else {
            palette::DIALOG_INK
        };
        let line = self::sentence(&mut commands, &fonts, &text, SENTENCE_PT, ink);
        commands.entity(middle).add_child(line);
    }

    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(baylee_client_core::ledge::BUTTON_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(middle).add_child(row);

    if let Some(words) = armed {
        // The armed card replaces the answers rather than joining them: the
        // first button *is* the answer, and the second is the way back.
        armed_row(&mut commands, &fonts, lang, &prefs, row, &words);
    } else {
        for (i, (says, label)) in answers.iter().enumerate() {
            let cap = if arrangement.density.shows_keycaps() {
                caps[i].as_deref()
            } else {
                None
            };
            // The candle is the first *answer*'s and stays there whatever
            // else joins the row: a command is never the thing the shelf is
            // inviting, and the invitation is what the candle is for. Which
            // is why this reads the `Says` and not only the index — the hold
            // row is one command standing alone at zero, and a burning "Ask
            // me again" would say the game is waiting for it.
            let weight = match says {
                Says::Command(super::MenuAction::ReleaseHold) => Weight::Ghost,
                Says::Answer(_) if i == 0 => Weight::Candle,
                Says::Answer(_) | Says::Command(_) => Weight::Secondary,
            };
            let button = answer(&mut commands, &fonts, label, weight, cap);
            match *says {
                Says::Answer(action) => {
                    commands.entity(button).insert(PromptButton {
                        action,
                        decision_id: duel
                            .interaction
                            .as_ref()
                            .and_then(baylee_client_core::Interaction::decision_id),
                    });
                    if clocked == Some(action) {
                        let (_, _, ink) = weight.colours();
                        let seconds = button_clock(&mut commands, &fonts, ink);
                        commands.entity(button).add_child(seconds);
                    }
                }
                Says::Command(action) => {
                    if action == super::MenuAction::ToggleGrantedActions {
                        let icon = commands
                            .spawn((
                                Text::new(super::glyph::HOURGLASS.to_string()),
                                super::icon_tf(&fonts, 13.0),
                                TextColor(palette::DIALOG_INK),
                                Pickable::IGNORE,
                            ))
                            .id();
                        commands.entity(button).add_child(icon);
                    }
                    commands.entity(button).insert(super::MenuButton { action });
                }
            }
            commands.entity(row).add_child(button);
        }
    }
}

/// Compact, framed category controls; icons and text share one hit target.
fn hand_tool(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    order: crate::hand_order::HandOrder,
    weight: Weight,
) -> Entity {
    use crate::hand_order::HandOrder;
    let mark = match order {
        HandOrder::Draw => baylee_client_core::tableicons::ZONES[0],
        HandOrder::Mana => '\u{f162}', // numeric ascending
        HandOrder::Name => '\u{f15d}', // alphabetic ascending
        HandOrder::Type => glyph::LIBRARY,
        HandOrder::Color => '\u{f53f}', // palette
    };
    let (fill, edge, ink) = weight.colours();
    let button = commands
        .spawn((
            Node {
                height: px(TOOL_H),
                align_items: AlignItems::Center,
                column_gap: px(TOOL_MARK_GAP),
                padding: UiRect::axes(px(TOOL_PAD_X), px(1)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
        ))
        .id();
    if let Some(feel) = weight.feel() {
        commands.entity(button).insert(feel);
    }
    let icon = commands
        .spawn((
            Text::new(mark.to_string()),
            table_icon_tf(fonts, mark, TOOL_MARK_PT),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    let label = commands
        .spawn((
            Text::new(label),
            tf_bold(fonts, TOOL_PT),
            TextLayout::no_wrap(),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(button).add_children(&[icon, label]);
    button
}
