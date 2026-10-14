//! Stack controls and the entry layout.

#[allow(clippy::wildcard_imports)] // the stack panel shares one drawing vocabulary
use super::*;
use crate::ui_button::UiButton;

pub(in crate::hud) fn control_button(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    active: bool,
    fonts: &UiFonts,
) -> Entity {
    let button = commands
        .spawn((
            UiButton,
            Node {
                min_height: px(28),
                padding: UiRect::axes(px(8), px(5)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(4)),
                border: UiRect::all(px(1)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(if active {
                palette::CANDLE_WASH_LIT
            } else {
                palette::DIALOG_LIT
            }),
            BorderColor::all(if active {
                palette::CANDLE
            } else {
                palette::DIALOG_LINE
            }),
            children![(
                Text::new(label),
                tf(fonts, 11.0),
                TextColor(palette::INK),
                Pickable::IGNORE
            )],
        ))
        .id();
    commands.entity(parent).add_child(button);
    button
}

pub(in crate::hud) fn pass_check(
    commands: &mut Commands,
    fonts: &UiFonts,
    enabled: bool,
) -> Entity {
    commands
        .spawn((
            Node {
                width: px(14),
                height: px(14),
                margin: UiRect::right(px(6)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor::all(palette::CANDLE),
            Pickable::IGNORE,
            children![(
                Text::new(if enabled {
                    glyph::CHECK.to_string()
                } else {
                    String::new()
                }),
                icon_tf(fonts, 10.0),
                TextColor(palette::CANDLE),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(in crate::hud) fn spawn_controls(
    commands: &mut Commands,
    panel: Entity,
    selected: Option<ObjectId>,
    orders: &[baylee_client_core::automation::AbilityOrder],
    lang: Lang,
    view: &PlayerView,
    fonts: &UiFonts,
    phone: bool,
) {
    use baylee_client_core::automation::{ability_order, set_ability_order};
    use baylee_engine::choice::StandingAnswer;
    let marked = selected.and_then(|id| view.stack.iter().find(|item| item.id == id));
    let ability = marked
        .or_else(|| view.stack.last())
        .and_then(|item| match item.stack_item {
            Some(baylee_view::StackItem::Ability { ability, .. }) => ability,
            _ => None,
        });
    // On a phone only the standing answers: the shelf already carries
    // "resolve the stack" (with the marked stop: `Duel::hold_action`), and
    // the hints are a desktop's room. Nothing left, nothing drawn.
    if phone && ability.is_none() && orders.is_empty() {
        return;
    }
    let controls = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
                // Under the list on a phone, and the first to give way: the
                // entry is what the panel is for (`fold_the_stack` keeps the
                // list a top row tall).
                flex_shrink: if phone { 1.0 } else { 0.0 },
                min_height: if phone { px(0) } else { Val::Auto },
                max_height: if phone { px(0) } else { Val::Auto },
                overflow: if phone {
                    Overflow::clip()
                } else {
                    Overflow::visible()
                },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if phone {
        // Folded until `fold_the_stack` has measured that they fit.
        commands
            .entity(controls)
            .insert((StackControls, Visibility::Hidden));
    }
    commands.entity(panel).add_child(controls);
    if !phone {
        let hint = if marked.is_some() {
            Phrase::StackStopHint
        } else {
            Phrase::StackSelectHint
        };
        let text = commands
            .spawn((
                Text::new(hint.text(lang)),
                tf(fonts, 10.0),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(controls).add_child(text);
        let running = view.priority_held;
        let label = if running {
            Phrase::HoldRelease
        } else if marked.is_some() {
            Phrase::StackRunTo
        } else {
            Phrase::StackRun
        };
        let button = control_button(commands, controls, label.text(lang), running, fonts);
        commands
            .entity(button)
            .observe(|mut click: On<PointerClick>, mut duel: ResMut<Duel>| {
                click.propagate(false);
                if let Some(action) = duel.hold_action(false) {
                    duel.submit(action);
                }
            });
    }
    if let Some(ability) = ability {
        let order = ability_order(orders, ability);
        let title = commands
            .spawn((
                Text::new(Phrase::StackAbilityPolicy.text(lang)),
                tf(fonts, 10.0),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(controls).add_child(title);
        let pass = control_button(
            commands,
            controls,
            Phrase::StackAlwaysPass.text(lang),
            order.pass,
            fonts,
        );
        let mark = pass_check(commands, fonts, order.pass);
        commands.entity(pass).insert_children(0, &[mark]);
        commands.entity(pass).observe(
            move |mut click: On<PointerClick>, mut prefs: ResMut<crate::prefs::Prefs>| {
                click.propagate(false);
                let mut order = ability_order(&prefs.all().ability_orders, ability);
                order.pass = !order.pass;
                set_ability_order(&mut prefs.edit().ability_orders, order);
            },
        );
        let choices = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(4),
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: px(4),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(controls).add_child(choices);
        for (label, answer) in [
            (Phrase::StackAsk, None),
            (Phrase::StackAlwaysYes, Some(StandingAnswer::Yes)),
            (Phrase::StackAlwaysNo, Some(StandingAnswer::No)),
        ] {
            let button = control_button(
                commands,
                choices,
                label.text(lang),
                order.answer == answer,
                fonts,
            );
            commands.entity(button).observe(
                move |mut click: On<PointerClick>, mut prefs: ResMut<crate::prefs::Prefs>| {
                    click.propagate(false);
                    let mut order = ability_order(&prefs.all().ability_orders, ability);
                    order.answer = answer;
                    set_ability_order(&mut prefs.edit().ability_orders, order);
                },
            );
        }
        if !phone {
            let hint = commands
                .spawn((
                    Text::new(Phrase::StackPolicyHint.text(lang)),
                    tf(fonts, 10.0),
                    TextColor(palette::MUTED),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(controls).add_child(hint);
        }
    }
    if !orders.is_empty() {
        let reset = control_button(
            commands,
            controls,
            Phrase::StackResetRules.text(lang),
            false,
            fonts,
        );
        commands.entity(reset).observe(
            |mut click: On<PointerClick>, mut prefs: ResMut<crate::prefs::Prefs>| {
                click.propagate(false);
                prefs.edit().ability_orders.clear();
            },
        );
    }
}

/// A row's height: a queued row's is fixed, which is what the virtual window
/// counts in; the full row's is its content, at least
/// [`STACK_FULL_HEIGHT`]. It was a fixed 164 px clip as well, and a two-line
/// name over four lines of sentence filled it before the targets were
/// reached (the owner, 08.10.2026). The sentence is a bounded box now, so
/// the content has a ceiling, and [`StackBody::full_height`] measures the
/// row for the window.
pub(in crate::hud) fn stack_entry_height(full: bool) -> Val {
    if full {
        Val::Auto
    } else {
        px(STACK_ROW_HEIGHT)
    }
}

/// One row of the stack panel: the object, what it is, and what it points at.
///
/// `full` is the top of the stack — the sentence the panel is about. Every
/// other row is the queue behind it and is drawn at two thirds, with no
/// subtitle and no arrow: a queued entry answers "what else is coming", and
/// the seat and the kind are questions a player asks about the thing that is
/// *next*.
#[allow(clippy::too_many_arguments)] // the same slot arguments, one level down
#[allow(clippy::too_many_lines)] // two shapes of one row, built flat
pub(in crate::hud) fn spawn_stack_entry(
    commands: &mut Commands,
    lang: Lang,
    item: &baylee_client_core::board::StackItem,
    full: bool,
    marked: bool,
    view: &PlayerView,
    statics: &GameStatic,
    picks: &Picks<'_>,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
    (text_lines, panel): (f32, PanelRoom),
) -> Entity {
    let phone = panel.phone;
    let key = StackKey::Entry(item.id, full);
    // The top row on a phone (`panel_room`): a queued row's card, the name
    // and its targets on one line, the sentence's box under them.
    let compact = full && phone;
    // The next thing to resolve is lit and railed; the one behind it carries
    // a hint of the same fill so "this resolves second" is visible, and
    // everything under *that* is flat. Depth is the only ordering a player
    // has to trust here, and past the second row it is carried by position.
    //
    // Under the pointer each of the three is one step lighter than what it
    // already had, which is how a hover can mean the same thing on rows that
    // are not peers. It is built into the tree rather than animated by a
    // [`Feel`], and that is particular to this panel: `ease_the_stack_in`
    // writes this very `BackgroundColor` for as long as a row is arriving,
    // and a second writer with its own opinion about the alpha would fight it
    // for a quarter of a second every time a spell is cast. The overlay is
    // rebuilt whenever the hover changes anyway — `HudRevision` counts it —
    // so the row can simply be *built* lit, which is what the hand zone has
    // always done with its halo.
    let hovered = picks.hovered == Some(item.id);
    let fill = if marked {
        palette::CANDLE_WASH_LIT
    } else if full {
        if hovered {
            palette::PANEL_HOT
        } else {
            palette::DIALOG_LIT
        }
    } else if item.depth == 1 {
        palette::DIALOG_LIT.with_alpha(if hovered { 0.62 } else { 0.45 })
    } else if hovered {
        palette::DIALOG_LIT.with_alpha(0.30)
    } else {
        Color::NONE
    };
    let rail = if full || marked {
        palette::CANDLE
    } else {
        Color::NONE
    };
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                height: stack_entry_height(full),
                min_height: px(if compact {
                    STACK_PHONE_FULL_HEIGHT
                } else if full {
                    STACK_FULL_HEIGHT
                } else {
                    STACK_ROW_HEIGHT
                }),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                column_gap: px(8),
                padding: UiRect::all(px(if full { STACK_PHONE_ROW_PAD } else { 4.0 })),
                border: UiRect::left(px(3)),
                align_items: AlignItems::FlexStart,
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(rail),
            // The whole row answers for the spell on it, and that is the fix
            // for a duel that could not be played: a `ChooseTargets` whose
            // only option was a spell on the stack had no way to be answered
            // at all, because the panel's one pickable node was the 66-pixel
            // picture and every other node — the row included — carried
            // `Pickable::IGNORE`. `Interaction::toggle` has always taken "a
            // spell on the stack"; what was missing was a way to *say* it.
            // A click anywhere on the row — the picture, the name, the
            // printed sentence — now goes through `activate_card` like a
            // click on a card in the hand, and lands on `toggle` for the
            // same reason it does there: nothing earlier in that chain is
            // true of an object on the stack.
            HandCardVisual { object: item.id },
            crate::hud::StackRowCard,
            Arriving::fill(key, fill.alpha()),
            ArrivingRow {
                lift: ARRIVE_LIFT,
                from: ARRIVE_SCALE,
                rail,
            },
        ))
        .id();

    let (width, height) = if full && !compact {
        (STACK_CARD_W, STACK_CARD_H)
    } else {
        (STACK_QUEUED_W, STACK_QUEUED_H)
    };
    let art = spawn_stack_card(
        commands,
        lang,
        key,
        // The row speaks for the entry; see `spawn_stack_card`.
        None,
        item.art,
        stack_face(item, view, faces, textures),
        width,
        height,
        statics,
        textures,
        assets,
        fonts,
        cards.as_deref_mut(),
        &faces.widths,
    );
    // The light that says the pending question would take this spell as its
    // answer, and the brighter one that says it already has. It replaces the
    // slot's drop shadow rather than joining it, exactly as in the hand:
    // `BoxShadow` is one component, and a card is lit or it is at rest.
    if let Some(light) = picks.light(item.id) {
        commands.entity(art).insert(light);
    }
    commands.entity(row).add_child(art);

    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                flex_grow: 1.0,
                min_width: px(0),
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(body);

    // The body is the panel less its padding, the row's own padding, the rail
    // and the card. The two rows spend it differently, and that is the whole
    // of the ramp once more: the **full** row's name may wrap to
    // [`STACK_NAME_LINES`], because it is the thing the row is about and nine
    // faces in the pool are longer than one line of it; a **queued** name is
    // cut to fit one line, because its height is what the panel is budgeted
    // against and at 267 px every face in the pool fits on it anyway.
    let ink = if full { palette::CANDLE } else { palette::INK };
    let size = if full {
        STACK_NAME_PT
    } else {
        STACK_QUEUED_NAME_PT
    };
    let room = panel.width
        - if phone {
            2.0 * STACK_PHONE_PAD
        } else {
            20.0
        }
        - if full { 12.0 } else { 8.0 }
        - 3.0
        - width
        - 8.0
        // On a phone's top row the targets share the name's line.
        - if compact {
            phone_targets_width(item.targets.len())
        } else {
            0.0
        };
    // The name a player reads rather than the one the engine projects. It has
    // to be looked up here and not taken from `item.name`, because
    // `BoardModel` is built in a crate that links neither the catalog nor a
    // socket to fetch it over.
    let title = view.object(item.id).map_or_else(
        || item.name.clone(),
        |o| crate::face::name_of(o, view, faces.texts),
    );
    let title = heading(item, full, title, faces);
    let mut name = commands.spawn((
        // A card's name is set in the card's face, not the interface's:
        // Faustina carries every printed word this client draws, wherever
        // the ground under it happens to be.
        super::tf_serif(fonts, size, 400),
        TextColor(ink),
        Arriving::ink(key, ink.alpha()),
        // The row is what the pointer is on, and a label is a node: left
        // pickable it would take the hover for itself and the row around
        // it would never light.
        Pickable::IGNORE,
    ));
    if full && !compact {
        name.insert((
            Text::new(title.clone()),
            bevy::text::LineHeight::Px(STACK_NAME_LINE),
            // A cap in pixels rather than a claim about the pool: a printing
            // this client has never seen gets two lines and a clean edge
            // instead of a third line that pushes the queue out of the panel.
            Node {
                max_height: px(STACK_NAME_LINES * STACK_NAME_LINE),
                overflow: Overflow::clip(),
                ..default()
            },
        ));
    } else {
        name.insert((
            Text::default(),
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
        ));
    }
    let name = name.id();
    if !full || compact {
        for piece in queued_heading_spans(&title, room, size) {
            let mark = piece.mark.then(|| piece.text.clone());
            let font = if piece.mark {
                crate::manaui::mana_tf(fonts, size * STACK_MARK)
            } else {
                super::tf_serif(fonts, size, 400)
            };
            let span = commands
                .spawn((
                    TextSpan::new(piece.text),
                    font,
                    TextColor(ink),
                    Arriving::ink(key, ink.alpha()),
                    Pickable::IGNORE,
                ))
                .id();
            if let Some(mark) = mark {
                crate::manaui::ink_span(commands, span, &mark, size * STACK_MARK);
            }
            commands.entity(name).add_child(span);
        }
    }
    if compact {
        // The name and what it points at on one line: the name takes what
        // the targets leave, cut to fit, and the targets never move off it.
        let line = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(6),
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(name).insert(Node {
            flex_shrink: 1.0,
            min_width: px(0),
            overflow: Overflow::clip_x(),
            ..default()
        });
        commands.entity(line).add_child(name);
        if !item.targets.is_empty() {
            let arrow = commands
                .spawn((
                    Text::new("→"),
                    tf(fonts, 14.0),
                    TextColor(palette::CANDLE),
                    Arriving::ink(key, palette::CANDLE.alpha()),
                    Node {
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            let targets = spawn_stack_targets(
                commands,
                lang,
                item,
                key,
                false,
                view,
                statics,
                textures,
                assets,
                fonts,
                faces,
                cards.as_deref_mut(),
            );
            commands.entity(targets).insert(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            });
            commands.entity(line).add_children(&[arrow, targets]);
        }
        commands.entity(body).add_child(line);
    } else {
        commands.entity(body).add_child(name);
    }

    // What kind of thing this is, and whose — on the top row only. An ability
    // names its source even when the source has left: the picture above may
    // be missing, the sentence must not be. A *spell* names nothing, because
    // the picture already said it. Not on a phone: its head says whose
    // answer the table waits for, and the row has two lines of room.
    if full && !compact {
        let kind = match item.kind {
            baylee_client_core::board::StackKind::Spell => None,
            baylee_client_core::board::StackKind::Ability { source, .. } => {
                Some(view.object(source).map_or_else(
                    || Phrase::StackAbilityBare.text(lang).to_string(),
                    |o| {
                        Phrase::StackAbility
                            .fill(lang, &[&crate::face::name_of(o, view, faces.texts)])
                    },
                ))
            }
        };
        let subtitle = commands
            .spawn((
                Text::default(),
                super::tf_serif(fonts, STACK_SENTENCE_PT, 400),
                TextColor(palette::MUTED),
                Arriving::ink(key, palette::MUTED.alpha()),
                Pickable::IGNORE,
            ))
            .id();
        if let Some(kind) = kind {
            let span = commands
                .spawn((
                    TextSpan::new(format!("{kind} — ")),
                    super::tf_serif(fonts, STACK_SENTENCE_PT, 400),
                    TextColor(palette::MUTED),
                    Arriving::ink(key, palette::MUTED.alpha()),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(subtitle).add_child(span);
        }
        // The seat in the slant, and only the seat: it is the one word of the
        // subtitle that names a *person*, and italic Inter at this size over
        // this ground is legible for a word and tiring for a sentence.
        let seat = commands
            .spawn((
                TextSpan::new(if item.controller == view.seat {
                    Phrase::You.text(lang).to_string()
                } else {
                    statics.seat_name(item.controller).to_string()
                }),
                super::tf_serif_italic(fonts, STACK_SENTENCE_PT, 400),
                TextColor(palette::MUTED),
                Arriving::ink(key, palette::MUTED.alpha()),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(subtitle).add_child(seat);
        commands.entity(body).add_child(subtitle);
    }

    // What it points at, right under what it is: on the full row above the
    // sentence, so a long sentence can never push its targets out of sight
    // (the owner, 08.10.2026: *"you can't see the target"*). The sentence
    // scrolls in its own box under them; the targets never scroll.
    if !item.targets.is_empty() && !compact {
        let targets = spawn_stack_targets(
            commands, lang, item, key, full, view, statics, textures, assets, fonts, faces, cards,
        );
        commands.entity(body).add_child(targets);
    }

    // What the ability *does*, in the player's own printing and language.
    // This is the whole reason the host says which sentence a stack entry
    // is: "+1" tells a player nothing, and a stack of three triggers that
    // all read `Ability · Ondu Cleric` tells them less.
    //
    // The full row only. A queued row answers "what else is coming", and six
    // sentences stacked under one another would be a wall of text where the
    // size ramp used to carry the order.
    if full && let Some(blocks) = stack_sentence(item, faces) {
        let (line, text_box) =
            spawn_stack_sentence(commands, fonts, key, item.id, blocks, text_lines);
        commands.entity(body).add_child(line);
        commands.entity(row).insert(StackTextRow { text_box });
    }

    row
}

/// How much of a phone's top row `count` targets take on the name's line:
/// the arrow and a queued row's thumbnails, with their gaps.
pub(in crate::hud) fn phone_targets_width(count: usize) -> f32 {
    if count == 0 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)] // a handful of targets
    let count = count as f32;
    6.0 + 14.0 + 6.0 + count * STACK_QUEUED_TARGET_W + (count - 1.0) * 4.0
}

/// A scrollbar's track and thumb on this panel: the list's and an entry's
/// sentence's are one look.
pub(in crate::hud) fn bar_colours() -> (Color, Color) {
    (
        palette::DOCK_EDGE.with_alpha(0.35),
        palette::CANDLE.with_alpha(0.8),
    )
}

/// How a scrollbar's colours go on here: as plain fills.
pub(in crate::hud) fn fill_bar(node: &mut EntityCommands, paint: super::ledge::log::Paint) {
    if let super::ledge::log::Paint::Fill(color) = paint {
        node.insert(BackgroundColor(color));
    }
}
