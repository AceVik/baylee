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
                min_height: px(24),
                padding: UiRect::axes(px(8), px(3)),
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
        .filter(|item| item.controller == view.seat)
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
                row_gap: px(3),
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
        commands.entity(button).insert(StackButton("StackRun"));
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
        commands.entity(pass).insert(StackButton("StackAlwaysPass"));
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
        for (tag, label, answer) in [
            ("StackAsk", Phrase::StackAsk, None),
            (
                "StackAlwaysYes",
                Phrase::StackAlwaysYes,
                Some(StandingAnswer::Yes),
            ),
            (
                "StackAlwaysNo",
                Phrase::StackAlwaysNo,
                Some(StandingAnswer::No),
            ),
        ] {
            let button = control_button(
                commands,
                choices,
                label.text(lang),
                order.answer == answer,
                fonts,
            );
            commands.entity(button).insert(StackButton(tag));
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
        commands
            .entity(reset)
            .insert(StackButton("StackResetRules"));
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
pub(in crate::hud) fn stack_entry_height(_full: bool) -> Val {
    Val::Auto
}

/// One row of the stack panel: the object, what it is, and what it points at.
///
/// `full` is the top of the stack — the sentence the panel is about. Every
/// other row is the queue behind it and is drawn at two thirds, with its
/// controller/source subtitle and any targets or cast/entering choices always
/// visible under its heading.
#[allow(clippy::too_many_arguments)] // the same slot arguments, one level down
#[allow(clippy::too_many_lines)] // two shapes of one row, built flat
pub(in crate::hud) fn spawn_stack_entry(
    commands: &mut Commands,
    lang: Lang,
    item: &baylee_client_core::board::StackItem,
    full: bool,
    marked: bool,
    group_info: Option<(ObjectId, usize)>,
    view: &PlayerView,
    statics: &GameStatic,
    reader: Option<PlayerId>,
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
        item.copy_original.or(item.art),
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
    if let Some(light) = picks.light(item.id) {
        commands.entity(art).insert(light);
    }
    commands.entity(row).add_child(art);

    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(if full { 3.0 } else { 2.0 }),
                flex_grow: 1.0,
                min_width: px(0),
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(body);

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
        - if group_info.is_some() { 54.0 } else { 0.0 }
        // On a phone's top row the targets share the name's line.
        - if compact {
            phone_targets_width(item.targets.len())
        } else {
            0.0
        };
    let title = view.object(item.id).map_or_else(
        || item.name.clone(),
        |o| {
            if (item.copy_original.is_some() || item.face_down)
                && let Some(card) = o.card
            {
                faces
                    .texts
                    .get(card.index, card.face)
                    .map(|t| t.name)
                    .or_else(|| {
                        baylee_cards::by_index(card.index)
                            .and_then(|d| d.faces.get(usize::from(card.face)))
                            .map(|f| f.name.to_string())
                    })
                    .unwrap_or_else(|| crate::face::name_of(o, view, faces.texts))
            } else {
                crate::face::name_of(o, view, faces.texts)
            }
        },
    );
    let title = if title.is_empty() || title == "Face-down" {
        match lang {
            Lang::De => "Verdeckt".to_string(),
            Lang::En => "Face-down".to_string(),
        }
    } else {
        title
    };
    let title = heading(item, full, title, faces);
    let mut name = commands.spawn((
        super::tf_serif(fonts, size, 400),
        TextColor(ink),
        Arriving::ink(key, ink.alpha()),
        Pickable::IGNORE,
    ));
    if full && !compact {
        name.insert((
            Text::new(title.clone()),
            bevy::text::LineHeight::Px(STACK_NAME_LINE),
            Node {
                max_height: px(STACK_NAME_LINES * STACK_NAME_LINE),
                overflow: Overflow::clip(),
                flex_grow: 1.0,
                min_width: px(0),
                ..default()
            },
        ));
    } else {
        name.insert((
            Text::default(),
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                overflow: Overflow::clip_x(),
                ..default()
            },
        ));
    }
    let name = name.id();
    if !full || compact {
        for piece in queued_heading_spans(&title, room.max(40.0), size) {
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
    let has_targets = super::cards::has_targets_or_choices(item, lang, full);
    if compact {
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
        if has_targets {
            let targets = spawn_stack_targets(
                commands,
                lang,
                item,
                key,
                (false, true),
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
            commands.entity(line).add_child(targets);
        }
        if let Some((group_id, count)) = group_info {
            let badge = spawn_group_badge(commands, fonts, key, group_id, count);
            commands.entity(line).add_child(badge);
        }
        commands.entity(body).add_child(line);
    } else if let Some((group_id, count)) = group_info {
        let head_row = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(6),
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let badge = spawn_group_badge(commands, fonts, key, group_id, count);
        commands.entity(head_row).add_children(&[name, badge]);
        commands.entity(body).add_child(head_row);
    } else {
        commands.entity(body).add_child(name);
    }

    // What kind of thing this is, and whose — shown on every row (both top
    // and queued entries). An ability names its source even when the source
    // has left: the picture above may be missing, the sentence must not be.
    if !compact {
        let sub_size = if full { STACK_SENTENCE_PT } else { 11.0 };
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
                super::tf_serif(fonts, sub_size, 400),
                TextColor(palette::MUTED),
                Arriving::ink(key, palette::MUTED.alpha()),
                Pickable::IGNORE,
            ))
            .id();
        if let Some(kind) = kind {
            let span = commands
                .spawn((
                    TextSpan::new(format!("{kind} — ")),
                    super::tf_serif(fonts, sub_size, 400),
                    TextColor(palette::MUTED),
                    Arriving::ink(key, palette::MUTED.alpha()),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(subtitle).add_child(span);
        }
        let seat = commands
            .spawn((
                TextSpan::new(if Some(item.controller) == reader {
                    Phrase::You.text(lang).to_string()
                } else {
                    statics.seat_name(item.controller).to_string()
                }),
                super::tf_serif_italic(fonts, sub_size, 400),
                TextColor(palette::MUTED),
                Arriving::ink(key, palette::MUTED.alpha()),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(subtitle).add_child(seat);
        commands.entity(body).add_child(subtitle);
    }

    // What it points at or chose on cast/entry, right under what it is: on
    // both the top row and every queued row.
    if has_targets && !compact {
        let targets = spawn_stack_targets(
            commands,
            lang,
            item,
            key,
            (full, false),
            view,
            statics,
            textures,
            assets,
            fonts,
            faces,
            cards,
        );
        commands.entity(body).add_child(targets);
    }

    if full && let Some(blocks) = stack_sentence(item, faces) {
        let (line, text_box) =
            spawn_stack_sentence(commands, fonts, key, item.id, blocks, text_lines);
        commands.entity(body).add_child(line);
        commands.entity(row).insert(StackTextRow { text_box });
    }

    row
}

fn spawn_group_badge(
    commands: &mut Commands,
    fonts: &UiFonts,
    key: StackKey,
    group_id: ObjectId,
    count: usize,
) -> Entity {
    commands
        .spawn((
            UiButton,
            StackButton("ToggleStackGroup"),
            StackGroupToggle,
            Node {
                padding: UiRect::axes(px(6.0), px(2.0)),
                column_gap: px(3.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(px(8.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::CANDLE_WASH_LIT),
            BorderColor::all(palette::CANDLE),
            Feel::tinting_to(palette::CANDLE_WASH_LIT, palette::PANEL_HOT),
            Arriving::fill(key, palette::CANDLE_WASH_LIT.alpha()),
            children![
                (
                    Text::new(format!("×{count}")),
                    tf(fonts, 11.0),
                    TextColor(palette::CANDLE),
                    Arriving::ink(key, palette::CANDLE.alpha()),
                    Pickable::IGNORE,
                ),
                (
                    StackGroupToggleText { group_id },
                    Text::new(glyph::CHEVRON_DOWN.to_string()),
                    icon_tf(fonts, 9.0),
                    TextColor(palette::CANDLE),
                    Arriving::ink(key, palette::CANDLE.alpha()),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .observe(
            move |mut click: On<PointerClick>,
                  mut fold: ResMut<StackFold>,
                  mut duel: Option<ResMut<Duel>>| {
                click.propagate(false);
                fold.toggle_group(group_id);
                if let Some(duel) = duel.as_deref_mut() {
                    duel.hovered = None;
                    duel.hovered_at = None;
                }
            },
        )
        .id()
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
